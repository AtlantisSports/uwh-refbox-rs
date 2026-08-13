# Third-Party Integration: Running Your Own Site Instead of the UWH Portal

Accurate as of refbox v0.4.9, plus the custom-source work that follows it and is **not yet in a
released build** — selecting a site inside the app, the per-site credential, and the narrowing of
call 2. Those parts are marked "unreleased" where they appear, and describe a branch that has not
shipped, so treat them as the intended shape rather than something you can test against a download
today. This is a best-effort description of what the software does; it carries no stability
promise, and a future release may change any of it without notice.

## Pointing refbox at your site

None of what follows matters until refbox is actually talking to your site. There are two routes,
they behave differently, and which one is open to you depends on which build you have:

- **A downloaded release** — the only thing available today. Use
  [The environment override](#the-environment-override-built-in-portal-only) below. The in-app
  route does not exist in your build, so skip the next section until it ships.
- **The unreleased custom-source branch** — use
  [The operator selects your site in the app](#the-operator-selects-your-site-in-the-app-unreleased),
  next. This is the route to prefer once it is released, which is why it is described first.

Everything after this section applies to both routes unless it says otherwise.

### The operator selects your site in the app (UNRELEASED)

In refbox's settings: set **MANUAL GAMES** to **NO**, choose **CUSTOM** as the source, tap the
**SITE:** row, type your address, and press **APPLY**. Nothing has to be set before launch, and
refbox is pointed at your site immediately — no restart.

The one address the operator types carries both halves refbox needs:

```
https://your-site/api/1234-A
```

| Half | Taken from | Used for |
|---|---|---|
| Base URL | everything before `/api/` | every path in this document is appended to it directly |
| Event ID | the segment after the marker — exact rule below | every call that names an event |

The exact rule, in order, because no one-line description predicts every case:

1. Split off the scheme and host. Everything after them is the path; the marker is only ever looked
   for there, never in the host.
2. In that path, find the **rightmost `/api/events/`**. If there is one, the event ID is what follows
   it. This is why `https://your-site/api/events/1234-A` yields `1234-A` and not `events`.
3. Only if there is no `/api/events/` at all, find the **rightmost `/api/`**, and the event ID is
   what follows that. Anything before the marker is kept as the base URL, so a site
under a path prefix works — `https://club.example/scoreboard/api/1234-A` gives a base of
`https://club.example/scoreboard`. An address with no host at all is refused rather than accepted.

The longer form `https://your-site/api/events/1234-A` is also accepted, and is what earlier builds
required. A `/api/` segment is mandatory in either form: without a fixed marker there is nothing
separating your site from the event ID, and an address like `https://your-site/login` would parse
as an event called `login`.

**That address is a refbox-side convention, not an endpoint you implement.** Nothing ever requests
`/api/1234-A`. It exists only so the operator can give refbox a site and an event in one field.
Your endpoints are the ones in this document — `/api/events/{id}/teams` and the rest — and they do
**not** move to `/api/{id}/…` just because that is what the operator typed.

**No `--allow-http` flag is needed for a site selected this way.** Whether TLS is demanded comes
from the scheme in the typed address, so a site on `http://` works by typing `http://`. This is the
practical reason to prefer this route for a box on the pool LAN.

Applying the address is not the end of the setup. refbox adopts the event named in it and
immediately requests your teams and your schedule — and the schedule is one of the calls that
needs a token, which it does not have yet. So expect the operator's first attempt to fail on your
side, then expect them to link (call 1, below) via the **ACCESS TOKEN** row, at which point refbox
re-requests the schedule by itself. **Refuse that first unauthenticated schedule request.** Serving
it is what lets the operator get as far as picking a game without ever pairing.

Two things refbox will refuse. Changing the address is rejected while a game is in progress, and
while any result is still queued and unsent — each with a message saying which, and the edit is
kept so it can be applied once the condition clears. And the credential is per-site: a token
issued by your site is stored against your site and is never sent to the UWH Portal, nor the
reverse.

### The environment override (built-in Portal only)

The older route replaces the address of the *built-in Portal*. Set an environment variable before
launching:

| refbox mode | Variable |
|---|---|
| Underwater hockey (6v6, 3v3, Beep Test) | `UWH_PORTAL_URL_OVERRIDE` |
| Underwater rugby | `UWR_PORTAL_URL_OVERRIDE` |

Set it to your site's base URL with no trailing slash — every path in this document is appended
to it directly. Which of the two variables applies depends on the mode refbox is configured for,
so setting the hockey variable while refbox is in rugby mode leaves it pointed at the real Portal,
with nothing to indicate the override was ignored.

**This override applies only when the selected source is the UWH or UWR Portal.** An address typed
into the SITE row is never redirected by it, which is deliberate: a typed address that was silently
ignored in favour of an environment variable is the failure the in-app route exists to remove.

**If your site serves plain `http://`, refbox refuses to send it anything by this route** unless it
is also started with the `--allow-http` flag. Without that flag no request is attempted at all, and
the failure is indistinguishable from your server being unreachable — nothing names the scheme as
the cause (`refbox/src/main.rs:667` sets it; `uwh-common/src/uwhportal/mod.rs:173` enforces it). A
site without a TLS certificate therefore needs both:

```bash
UWH_PORTAL_URL_OVERRIDE=http://localhost:8099 refbox --allow-http
```

### One condition that applies to both routes

Easy to lose an afternoon to: refbox must be in one of its **game** modes — 6v6, 3v3 or Rugby — for
any of this to happen at all. Beep Test mode still constructs the Portal client during startup, but
nothing on that screen ever calls it, so no request arrives and there is nothing to link.

The mode is changed on the **APP MODE** button in refbox's settings, which restarts the app; there
is no command-line option for it. Moving between hockey and rugby moves refbox between the two
Portal tenants and invalidates a link to the built-in Portal, so an operator on the Portal has to
link again afterwards. A site selected in the app is unaffected — it keeps both its address and its
token across that restart, and needs no re-linking.

## You probably need nine calls, not eighteen

The refbox application, the pre-tournament schedule tool, and the stream overlay together
make eighteen different calls to the UWH Portal. If you only want to run a site that stands
in for the Portal on the day of the tournament — the thing the referee's box actually talks
to at poolside — you only need to support nine of them:

| # | Operation | Path | Auth | Inventory # |
|---|---|---|---|---|
| 1 | Link a refbox | `POST /api/events/{eventId}/access-keys/ref-box` | none | 1 |
| 2 | Verify token | `GET /api/events/{eventId}/access-keys/verify` | bearer | 3 |
| 3 | Event list | `GET /api/events` | none | 9 |
| 4 | Event teams | `GET /api/events/{eventId}/teams` | none | 8 |
| 5 | Schedule (privileged) | `GET /api/events/{eventId}/schedule/privileged` | bearer | 6 |
| 6 | Referees | `GET /api/events/{eventId}/referees` | none | 7 |
| 7 | Push scores | `POST /api/events/{eventId}/schedule/games/{gameNumber}/scores` | bearer | 5 |
| 8 | Push stats | `POST /api/admin/events/stats` | bearer | 4 |
| 9 | Team roster | `GET /api/admin/get-event-team` | none | 13 |

**These nine numbers are the ones this document uses.** Every later reference to "call 5" and
the like means this table. The Full inventory below numbers all eighteen calls in source order
instead, so its numbers do not match — the last column above is the bridge between the two.

The other nine calls serve two separate programs that are not the refbox itself: the
pre-tournament admin tool (`schedule-processor`), which uploads and manages a schedule before
the tournament starts, and the stream overlay, which pulls attachments for the video overlay.
If you're only standing up something for the refbox to talk to during a game, you can ignore
those nine.

Call 9 is the one to watch. The other eight announce themselves when they fail — a refusal, a
red indicator, a queued result that will not send. Call 9 fails silently: the refbox carries on
as though nothing is wrong, and the only sign is a feature the operator quietly does not get.
Its entry below spells out exactly what is lost.

## Full inventory

All eighteen calls the refbox ecosystem makes to the Portal today, across all three programs.
"Auth" is "bearer" when the call requires a bearer token in the `Authorization` header, and
"none" when it does not.

**The numbers in this table are source order and are used nowhere else in this document.** When
the text says "call 2" it means the nine-call table above, not this one. Match rows by path, or
use the "Inventory #" column above.

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
| 13 | GET | `/api/admin/get-event-team` | refbox + schedule-processor + overlay | none | `uwh-common/src/uwhportal/mod.rs:671` |
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

## The refbox nine

Full detail on the nine calls a stand-in site must answer, in the same order as the table
above. Every entry uses the same headings so you can skim them side by side. Two general rules
that apply across all nine, spelled out here because they matter more than any single field:

- An event ID in a URL **path** is always the short form (`1234-A`). An event or team ID in a
  **query parameter** is always the long form (`events/1234-A` or `teams/5678-B`). Two of the
  nine use the long form, and both only because the ID is a query parameter: push stats
  (`eventId`) and team roster (`teamId`). The
  [Data formats](#data-formats) section documents this in full; it's mentioned here because it
  affects calls 8 and 9 below.
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
call in the nine that's a back-and-forth conversation rather than a single request.

**Authentication:** none

**Query parameters:** none

**Request body:** `{"refBoxId": "<string>", "code": "<string>"}` — note both values are sent
as JSON strings, not numbers, even though both are made of digits. Example:
```json
{ "refBoxId": "482913", "code": "731904" }
```

**The `code` you issue must obey three rules.** The operator types it on refbox's numeric keypad,
which holds the value as a *number* and only then sends it as text — so:

- **Digits only.** There are no letters on the keypad.
- **At most six digits.** The keypad refuses input above `999999`.
- **No leading zeros.** A code of `042913` is typed as a number and reaches you as `"42913"`, which
  will not match what you issued. This is the trap: zero-padding to a fixed width is the most
  natural way to mint a six-digit code, and it makes linking impossible rather than merely
  awkward — the operator gets `InvalidCode` every time, with no other way to install a credential.

**Successful response:** `200` with `{"accessKey": "<token>"}`. Example:
```json
{ "accessKey": "a1b2c3d4e5f6" }
```

**Keep the `accessKey` to printable ASCII, with no whitespace and no control characters.** refbox
puts it directly into an HTTP header without checking it, so a character that is illegal there
**crashes the application** rather than being reported as a bad token. It happens almost
immediately: refbox requests the schedule as soon as a link succeeds, so the crash follows the
link within seconds, in front of the operator who just linked. There is no length limit.

**One credential per site, honoured for every event.** The event id in this path names the event the
operator was on when they linked; it does not scope the key. refbox stores exactly one credential
per site and presents it again for every other event on that site — switching event does not
re-link. **A site that binds a key to the event it was issued under will start refusing everything
the moment an operator changes event**, mid-tournament, with results possibly still queued.

**Fields refbox actually reads:** `accessKey` on success; `reason` on a `400`. Nothing else in
the response is read.

**On failure:** See the worked example below for the two specific `400` reasons refbox
recognises. Any other status code, or a `400` whose body doesn't contain a recognised `reason`,
is treated as an unexpected error: refbox logs it and leaves the operator on the code-entry
screen with no visible message — there is no third error state shown in the UI.

##### The login flow, step by step

This is the only one of the nine that's a conversation instead of a single call:

1. refbox generates a random number between 1 and 999999 once, the first time it's needed, and
   reuses it for the rest of that run (`mod.rs:199`). This is the `refBoxId`. **It may be shorter
   than six digits** — a real refbox can show the admin `42` — and it is never zero-padded, so do
   not validate it as a six-digit string. **It is also regenerated every time refbox restarts**, so
   treat it as a one-time pairing number, not a device identity: you cannot pre-register boxes the
   night before, and you cannot use it to trace which box pushed which result.
2. An admin on the tournament site enters that number into the site, which issues a short code.
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

##### The whole exchange, end to end

Every other non-trivial shape in this document has a worked example; this one is a conversation, so
here it is in full. `1234-A` is the event, `482913` is what the operator reads off the refbox
screen, and `731904` is the code your site mints.

**Step A — the admin registers the box.** Your own screen, your own shapes; refbox is not involved
and this document does not specify it. You record a pending link for `482913` and show the admin
a code:

```
refBoxId 482913  →  code 731904   (six digits, no leading zero, expires when you decide)
```

**Step B — the operator types `731904` into refbox, which posts:**

```http
POST /api/events/1234-A/access-keys/ref-box
Content-Type: application/json

{ "refBoxId": "482913", "code": "731904" }
```

**Step C — you answer.** Success, consuming the pending link:

```http
200 OK
Content-Type: application/json

{ "accessKey": "a1b2c3d4e5f6" }
```

Wrong code, right box:

```http
400 Bad Request

{ "reason": "InvalidCode" }
```

No admin ever registered that box — or the code expired and you dropped the pending link:

```http
400 Bad Request

{ "reason": "NoPendingLink" }
```

Those two strings are matched literally and each shows the operator a different message. Anything
else — a third `reason`, a missing `reason`, a `401`, a `500` — leaves them on the code-entry
screen with no message at all.

**Step D — from here on**, refbox sends `Authorization: Bearer a1b2c3d4e5f6` on every call marked
"bearer", **including for other events on your site**. It will not link again unless the operator
asks it to.

**Implement the negotiation. Do not shortcut it.** refbox cannot make you: it only ever checks a
token by sending it as a bearer header (call 2) and never re-derives it from the login response, so
a site *could* answer call 1 with any string at all and accept that string as a bearer token
afterwards. Nothing in calls 2–8 would notice. That is a description of refbox's limits, not
permission — a site that hands a working token to anyone who posts to call 1 is an open token
dispenser, and it hollows out the requirement in call 2 that you reject tokens you did not issue.
Since refbox enforces nothing here, your site is the only thing standing between an event's results
and whoever can reach it.

Concretely, what implementing it means:

- Record the `refBoxId` an admin entered, so a pending link exists before any code is issued.
- Issue a code bound to that `refBoxId`, and accept it only for that `refBoxId`.
- Reject anything else with `400` and one of the two exact `reason` strings from step 6 above —
  `NoPendingLink` when no admin has entered that `refBoxId`, `InvalidCode` when the code does not
  match.
- Expire codes, and make a code single-use — consume the pending link when it is redeemed. A
  replayable code is a smaller version of the open token dispenser above. **How long a code lives
  is your choice** (long enough for an admin to walk the pool deck and read it out), and so is
  what you do when the code and `refBoxId` are right but the admin registered them against a
  different event. Only two `reason` strings exist, so an expired or lapsed code has to be one of
  them: use `NoPendingLink`, which matches its own definition and tells the operator to ask for a
  fresh registration.
- **The admin half of this handshake is yours entirely** — how an admin enters a `refBoxId`,
  what that screen looks like, who may reach it, how it authenticates. refbox never touches it, so
  nothing here constrains it and nothing can verify it. This document deliberately says nothing
  more about it; that silence is not an omission.
- Keep every `accessKey` revocable on your side. **refbox never gives one up.**
  Nothing in the app clears a stored token — the only caller of the client's `clear_token` in the
  whole workspace is schedule-processor, not refbox — so a key you issue stays in that refbox until
  a fresh login overwrites it or somebody edits the configuration file. If a refbox is lost or
  retired, revoking your side is the only way to end its access.

What you cannot do is bypass call 1 altogether by having the operator type a token in. **refbox has
no way to enter a token.** The custom-site row (unreleased) takes a site *address*, not a
credential; the linking screen still offers a numeric keypad only, and the field it fills is the
six-digit-style `code` — not the `accessKey`. A token can only be installed by editing refbox's
configuration file on disk — `custom_site.token` for a site selected in the app, `uwhportal.token`
for the built-in Portal — which is not something an operator does mid-tournament. Plan on answering
call 1.

Two further things about this exchange that are invisible from the request and response alone, and
that both showed up the first time a real refbox was pointed at a stand-in site:

**Your site cannot change what the operator is told to do.** refbox's on-screen linking
instructions are fixed text: they tell the operator to go to "Portal >> Event Management >> Referee
Management, click on the + button to add a new Refbox", and to expect a confirmation code back
(`refbox/translations/en-US/refbox.ftl:177-181`). Only the product word — "UWH" or "UWR" — varies,
and it varies with refbox's game mode, not with the site it is talking to. An operator pointed at
your site is therefore given a menu path that does not exist for them. You cannot override this
text, so whatever admin flow you build for issuing codes, expect to document it yourself and expect
operators to arrive confused. Note also that refbox only lets the operator reach this screen once an
event has been selected, so a code has to be obtainable for an event the operator has already
chosen.

**A successful call 1 is trusted without being verified.** On receiving an `accessKey`, refbox
marks the token valid immediately (`refbox/src/app/mod.rs:5110-5112`) rather than confirming it with
call 2. The portal indicator turns green on the strength of your response alone. If your site issues
a key it will not subsequently honour, refbox will show a healthy green portal until the standing
health check notices — up to five minutes later.

#### 2. Verify token

`GET /api/events/{eventId}/access-keys/verify`  ·  source: `uwh-common/src/uwhportal/mod.rs:300`

**When refbox calls it:** Twice, for different reasons, and **only ever while it holds a token**
(narrowed in the unreleased custom-source work — see below). First, whenever the operator opens
Game Options with a token
already saved, or picks/changes the event there — refbox checks the token before showing the portal
settings as usable. Second, automatically in the background as a standing health check, for as long
as an event is selected: every 5 minutes while everything is healthy, dropping to every 15 seconds
once a problem is detected (so it notices a recovery quickly).

Holding no token, refbox now reports the credential as failed without sending anything, on both
paths (`refbox/src/app/mod.rs` for the settings row, `refbox/src/portal_manager/mod.rs` for the
health check). This is the one endpoint of the four bearer calls that behaves this way; the other
three still arrive unauthenticated, as described in the fourth rule under "Rules that apply to
every call".

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** none

**Request body:** none

**Successful response:** `200`. The body is never parsed — refbox only checks the status code, so
a genuinely empty body works, and so does `{}` (the safe default recommended under
[Rules that apply to every call](#rules-that-apply-to-every-call)) — nothing here ever looks at it
either way.

**Fields refbox actually reads:** none.

**On failure:** Any non-`200` response, or a request that never completes at all (no network,
DNS failure, timeout), counts as failure — but refbox tells the two apart for this call
specifically. A dropped connection ("the site is unreachable") turns the portal status
indicator red *without* asking the operator to log in again, because the saved token might
still be perfectly valid. An HTTP response that isn't `200` (a `401`, or anything else) is
treated as "the token itself is bad": the indicator goes red *and* the operator is prompted to
log in again. This distinction is unique to this call; calls 7 and 8 below treat both kinds of
failure the same way.

**What your site must do — the one obligation you cannot delegate to refbox.** Reject any token
you did not issue, with any non-`200`. Your site is the only thing in the system that enforces
whether a given refbox is authorised for an event, and this call is the only place that
enforcement is visible to refbox.

**"A token you did not issue" means exactly that, and nothing more.** The event ID in this path is
not a scope: a key you issued under one event must still be accepted here for every other event on
your site. refbox holds one credential per site and re-presents it whenever the operator changes
event — see call 1. Rejecting on the event rather than on the key is the single easiest way to build
a site that passes every test and then fails mid-tournament.

The reason this is stated as an obligation rather than a suggestion was observed live on 2026-08-10
against a deliberately permissive stand-in site. A site that answers `200` to everything tells
every refbox pointed at it that its token is already valid. refbox then behaves as though it were
already paired: the token row shows `OK` in green, the privileged schedule loads, the court fills
itself in, and **the operator is never offered the link flow at all**. Pairing is bypassed
completely, and nothing anywhere reports a problem — not the screen, not the log. That is the
failure mode reading refbox's source cannot reveal, because the source shows only what refbox
*sends*, never what a site is obliged to *refuse*.

**Unreleased: refbox has closed the worst version of that, and only the worst version.** It no longer
asks you to vouch for a credential it does not have, so the specific case above — a bypass achieved
with no token at all — is no longer reachable through this call: refbox reports the credential as
failed itself rather than believing your `200`. Verified against a permissive stand-in site on
2026-08-11, which received no verify request whatsoever.

What that does **not** cover is a token you never issued: a stale one from another site, an expired
one you have since revoked, or a fabricated one. refbox holds such a token and will send it, and if
you answer `200` it is authorised for the event on your say-so. The obligation is unchanged, and so
is its consequence — you are still the only thing enforcing it. Note also that refbox marks a token
valid the moment call 1 returns it, without confirming it here, so a key you issue and then decline
to honour shows a healthy green until the standing health check catches up.

**What a revoked key should return, and how the operator gets back in.** Revoking a key on your
side just means this call starts returning something other than `200` for that token — the status
check above (`uwh-common/src/uwhportal/mod.rs:314`) does not distinguish a `401` from any other
non-`200` code, so a `403` or a `500` produces the identical result to a textbook `401`: per "On
failure" above, the indicator goes red and the operator is prompted to log in again. What does
**not** work is revoking by dropping the connection instead of answering it — refusing the socket,
or simply not listening — because that is indistinguishable from your site being down, and refbox
classifies it as **unreachable** rather than as a rejected token: red, but without the re-login
prompt. The operator can still recover either way, because getting back in doesn't depend on this
call at all: the event picker (call 3) and the login call (call 1) both need no token, so the
operator can still select an event and reach the login screen with a fully revoked credential in
hand, and re-link exactly as on first setup — the button that opens it is gated on nothing but a
selected event (`refbox/src/app/view_builders/configuration.rs:828-832`). Answer a revocation with
an HTTP status, not a closed socket, so the operator is actively told to log in again rather than
left to notice the red indicator on their own.

#### 3. Event list

`GET /api/events`  ·  source: `uwh-common/src/uwhportal/mod.rs:537`

**When refbox calls it:** At startup, if UWH Portal mode is already turned on, and again
whenever the operator turns "Use UWH Portal" on from off in Game Options.

**Authentication:** none

**Query parameters:** `limit` (always the literal string `"100"`), `filter` (`"Past"` or
`"InProgressOrUpcoming"`, from an operator setting for whether to include past events), and
`isSchedulePublished` (always `"true"` — refbox only ever asks for events whose schedule has
been published).

Your site is expected to apply all three of those itself. refbox does no filtering of its own —
it displays every entry you return, exactly as returned — so an unfiltered list puts every past
tournament in the operator's event picker. It also never asks for a second page: there is no
offset, page, or cursor parameter anywhere in this call, and `totalCount` is read but never acted
on. If more than 100 of your events match, the remainder are simply unreachable from the picker.

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
event picker), `slug` (must be present as a JSON string to parse successfully, but nothing requires
it to be non-empty — `"slug": ""` parses exactly as well as any other value, since it's a plain
`String` field with no length or format check, unlike `EventId`/`TeamId`
(`uwh-common/src/uwhportal/schedule.rs:791`) — and refbox never displays or acts on it either way),
and `dateRange.startsOn` / `dateRange.endsOn` (used only to sort the picker, earliest
tournament first — never displayed). **`dateRange` itself is required, with both fields** — unlike
the three below it is not optional, and `null` or a missing key fails that entry, which costs you
the whole event list rather than the one event. `teams`, `schedule`, and `courts` may be omitted
entirely:
refbox fills those in itself via calls 4 and 5, per event, right after this call returns. **There
is no court list to serve anywhere** — refbox builds one by collecting the distinct `court` values
across the games in call 5's schedule, sorted. A court with no games scheduled on it therefore
cannot be offered to the operator, and an event whose games are all on one court auto-selects it.

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

**Authentication:** `Authorization: Bearer <token>` — **and refusing it when that header is
absent or carries a token you did not issue is your obligation, not a formality.** refbox sends
this call unauthenticated as a matter of course: the operator points it at your site, refbox
adopts the event and asks for the schedule immediately, before any pairing has happened. Answer
that with `200` and the operator can pick a court and run games on your data without ever
linking, and nothing anywhere on their screen will say so. Return `401`; refbox recovers by
itself once the operator links and re-requests the schedule. See
[Rules that apply to every call](#rules-that-apply-to-every-call).

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
`tournamentReferee` (a single object, `null`, or absent — all three parse) and `referees.dedicated` /
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

**Put plainly, the minimal valid body is `{}`.** Every field above is optional — `tournamentReferee`
and all three `referees.*` arrays alike — and the parse this call runs is a generic
`serde_json::Value` parse that succeeds on any valid JSON at all (`uwh-common/src/uwhportal/mod.rs:468`).
An empty object satisfies every requirement here at once; it just means refbox finds no referees to
name.

Note what is **not** read: `user.name` is never used, even when it is the only name an entry
carries. That is deliberate — it holds the official's real name, and refbox prefers a chosen
handle for an operator-facing screen. So a site that returns `user.name` and nothing else will
show no referee names at all, and nothing will be logged to explain why. Populate `rosterName`
or `user.username`.

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

**The UWH Portal does reject a mismatch — that is why the FORCE button exists.** The paragraph above
describes what `force` means to a site that rejects. **A third-party site is advised not to be such
a site**, for the reasons below. If you take that advice, `force` carries no extra meaning against
your site and FORCE behaves exactly like RETRY, which is the intended outcome: there is nothing for
the operator to force past.

**Store whatever score arrives, and refuse only when you genuinely cannot save it.** It is tempting
to reject a re-pushed score that differs from one you already hold, and to make the operator use
FORCE — but refbox cannot tell your rejection from your site being unreachable. A result you refuse
goes onto the local queue, retries every 15 seconds, is flagged to the operator after 30 minutes,
and is **archived and dropped after 120 hours**. Being wrong in the permissive direction costs an
overwritten score, which somebody can see and correct; being wrong in the strict direction costs the
result entirely, quietly, while the operator is turning to the next game. The same reasoning applies
to a score for a game number you do not recognise: store it rather than refuse it.

On this call
`force` is **always present**, as the literal `true` or `false` — never omitted
(`uwh-common/src/uwhportal/mod.rs:367`). One of the other-nine calls that also takes `force`
behaves differently; see the schedule upload in [Full inventory](#full-inventory) (inventory #10)
— not "call 8" of the refbox nine, which is push stats.

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

**Query parameters:** `eventId` (the event ID, **long form** — `events/1234-A` — one of the two
exceptions among these nine to the short-form-in-path rule, the other being call 9's `teamId`,
since this is a query parameter, not a path segment) and `gameNumber` (the game's number as a
plain string, e.g. `"3"`).

**Request body:** A bare JSON array of event objects — the game's goals, penalties, and fouls —
for the game that just ended. This is the other large, shared request shape — see
[Data formats](#data-formats) for the exact fields.

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

#### 9. Team roster

`GET /api/admin/get-event-team`  ·  source: `uwh-common/src/uwhportal/mod.rs:671`

**When refbox calls it:** Twice, both times on its own initiative — the operator never asks for
this call and never sees it happen. First, whenever a schedule arrives: once for every distinct
team assigned anywhere in that schedule, skipping any team whose roster is already cached. On a
full tournament schedule that is a burst — a comment on the call warns against re-firing "~40
concurrent GETs" (`refbox/src/app/mod.rs:4993`), so expect a few dozen near-simultaneous requests
on first load and size your site accordingly. Second, a refresh for the two teams of the upcoming
game, fired at the end of the previous game so the fetch has the whole break to land rather than
the instant of the next start (`refbox/src/app/mod.rs:1355`).

**Authentication:** none — and genuinely so, not merely "no token required": this call never
carries an `Authorization` header at all, even when refbox holds a valid one (see
[Rules that apply to every call](#rules-that-apply-to-every-call)).

**Query parameters:** `teamId` — the team ID, **long form** (`teams/5678-B`), per the
query-parameter rule under [the two ID forms](#the-two-id-forms).

**Request body:** none

**Successful response:** `200` with a `roster` array. Example:
```json
{
  "roster": [
    { "rosterName": "Casey", "capNumber": 7, "roles": ["Player", "Captain"] },
    { "rosterName": "", "username": "reef_ref", "capNumber": 12, "roles": ["Player"] }
  ]
}
```

**Fields refbox actually reads:** in effect only `capNumber` — but two filters decide which
entries survive to be read at all.

The first is on `roles`, and it is an **inclusion** test, not an exclusion one: an entry is kept
if its `roles` array contains any of `"Player"`, `"Captain"` or `"ViceCaptain"`, and dropped
otherwise (`uwh-common/src/uwhportal/mod.rs:121`). Any other role sitting alongside a playing one
is simply ignored, so a team member listed as both `"Player"` and `"Coach"` **does** appear on the
grid — someone who coaches and plays is still a player. Only a member with no playing role at all
— a `"Coach"`, `"Manager"` or `"Official"` and nothing else — is dropped. Do not implement this as
"exclude anyone labelled Coach"; that would hide playing coaches from the operator.

The second filter is on the number: of the entries that survive the role test, refbox keeps only
cap numbers in the range **1 to 99**; a `capNumber` of `0`, or of `100` or more, is silently
discarded (`refbox/src/app/mod.rs:6332`). Names are parsed but refbox itself ignores them — a
site serving only refbox can return `capNumber` and `roles` alone.

That's the *sufficient* field set, not the *required* one, and the two differ in a way worth
knowing precisely. `roles` is required unconditionally: an entry with no `roles` key at all never
sets any of the three role flags — `member.get("roles")` simply returns `None`
(`uwh-common/src/uwhportal/mod.rs:104`) — so it fails the role test above and is dropped, no
matter what else it carries, including a perfectly good `capNumber`. `capNumber` is not required
in that same unconditional way: an entry that clears the role test is kept if it has *either* a
non-empty display name *or* a numeric `capNumber` — only an entry with neither is dropped
(`uwh-common/src/uwhportal/mod.rs:125`). That's also what a **string** `capNumber` costs you.
`"capNumber": "7"` is not read as `7`: `.as_u64()` returns `None` for a JSON string exactly as it
would for a field that's absent (`uwh-common/src/uwhportal/mod.rs:87`), so nothing errors and
nothing is logged. If the entry still has a name, it's kept — but as an **unnumbered** player,
sorted after every numbered one — and an unnumbered player never becomes a button: refbox skips
any roster entry with no cap number when it builds the grid, because there is nothing to tap
(`refbox/src/app/mod.rs:6332`). A string `capNumber` — the shape a naive CSV export produces by
default — therefore never surfaces as an error. It quietly removes one player from the grid, the
same way the whole call failing removes all of them.

**On failure:** **nothing visible happens.** Any non-`200` response, or a body that doesn't parse,
is written to the log and otherwise discarded, and the failure deliberately leaves any previously
cached roster untouched rather than replacing it with an empty one
(`refbox/src/app/mod.rs:935`). There is no retry, no queued item, no indicator change, and no
message to the operator.

What the operator loses is the **player-number grid**. Given a roster, the pages that attribute an
action to a player — goals, penalties, fouls, warnings — show that team's cap numbers as a grid of
tappable buttons. Given no roster, they fall back to a plain number pad and the operator types
each cap number by hand. Both routes work, which is exactly the problem: a site that never answers
this call produces a refbox that looks entirely healthy, and is simply slower to operate for the
whole tournament, with the only evidence a log line nobody is reading. Of these nine calls, this
is the one whose absence nothing will tell you about.

**What the other two callers read:** the same response serves schedule-processor and the overlay,
and both read more of it than refbox does. `schedule-processor` calls it while generating
scoresheets, once per team, and only for the scoresheet styles that print a roster; per entry it
reads `capNumber`, a display name (`rosterName` when non-empty, otherwise `username`, otherwise
blank), and `roles` — applying the same playing-role filter as above, and additionally marking
`"Captain"` / `"ViceCaptain"` on the printed form. It drops any entry with neither a name nor a
cap number, and sorts numbered entries first by ascending number, then unnumbered ones
alphabetically. On failure it logs a warning and treats the team as having an empty roster,
continuing with a blank roster rather than stopping. The overlay reads different fields again,
including top-level `name` and `logoUrl`; see
[The overlay's other calls](#the-overlays-other-calls-same-paths-different-code).

## Data formats

This section gives the exact field-by-field shape of everything the calls above only summarised:
the rules every response has to satisfy, the two ways an ID can be written, the schedule the
refbox downloads, the two timestamp formats, and the stats records refbox uploads after a game.

### Rules that apply to every call

Four rules hold across the whole API and are invisible from the individual call descriptions
above, because each one only becomes apparent when you compare all of them.

**The HTTP-level facts, in one place.** refbox gives every request **10 seconds** before treating it
as a transport failure, so a site that takes longer to answer is indistinguishable from a site that
is down. Redirects, keep-alive and connection reuse are whatever the underlying HTTP client does by
default — nothing is configured, so do not build a site that depends on a particular behaviour
there. Requests are not serialised: selecting an event triggers a burst (teams and schedule
together), so a single-threaded stand-in can stall its own startup. Answer promptly, and answer
concurrently.

**Exactly `200` counts as success.** Every call made through the shared Portal client — that is,
every call in this document except the overlay's own — compares the response status against
`200 OK` itself, never against the `2xx` range. There is no `is_success()` check anywhere in it,
and `201`, `202` and `204` are not recognised as success. A site that answers a score push with
`204 No Content` has that push treated as a **failure**: it goes back on the local queue and is
retried about every 15 seconds until it eventually archives, with nothing on screen explaining
why. Return `200` — `{}` is the safe default body — rather than `204`.

**Whether the body's content matters at all depends on the call.** Three calls never parse the
response body on success, at all: verify token (call 2), push scores (call 7), and push stats
(call 8) — see their own entries. For those three, a genuinely empty body works exactly as well
as `{}` does. The other six calls do parse the body and need whatever fields their own entry
marks required; `{}` alone will not satisfy those.

Exactly one call reads any other status: call 1 of the refbox nine treats `400` as "that code was
wrong" and surfaces it to the operator as an invalid code
(`uwh-common/src/uwhportal/mod.rs:239`). Everywhere else, every non-`200` means the same single
thing, so the status you choose for "no such event" carries no meaning to refbox — `404`, `400`
and `500` are indistinguishable to it. An event deleted from your site therefore looks, to the
operator, identical to your site being down — same red indicator, no update, no way to tell "it
was removed" from "it can't be reached." The overlay is the opposite extreme: it never inspects the
status code at all, and simply attempts to parse whatever body comes back.

**Unknown fields are ignored, everywhere.** No type in this API sets serde's
`deny_unknown_fields`, so extra fields you include in a response are discarded silently —
including inside the large schedule shape, where a hard parse failure would otherwise cost you a
schedule that never loads. You can extend responses freely. What you cannot do is rename or omit
anything a shape marks as required.

**refbox always sends a fixed `Content-Length`, never a chunked body,** and always sets
`Content-Type: application/json` on requests that have one. Two different mechanisms produce that,
and both are worth knowing if you are debugging raw traffic: most bodies go through reqwest's
`.json()` helper, which sets the header and the length together (for example
`uwh-common/src/uwhportal/mod.rs:221`), while push stats serialises its body itself and sets the
header explicitly (`mod.rs:335`). What arrives on the wire is the same either way.

refbox does not require any particular `Content-Type` on your responses. Every response body is
read as text and then parsed as JSON regardless of how you labelled it. A hand-written server
reading the request body straight off the socket can rely on `Content-Length` being present and
accurate.

**refbox still makes three of the four authenticated calls when it holds no token.** Having no
token does not stop refbox from calling 5, 7 and 8 — it makes them anyway, with the `Authorization`
header **omitted entirely**, not sent as an empty `Bearer `. An empty token in its configuration is
turned into "no token" (`refbox/src/app/mod.rs`), and the request builder attaches the header only
when a token is actually present (`uwh-common/src/uwhportal/mod.rs:849`). Nothing checks in between
on those three paths.

**Call 2 is the exception, in the unreleased custom-source work:** it is sent only while refbox
holds a token. Both places
that make it — the settings row and the background health check — now check first and report the
credential as failed themselves rather than asking your site to rule on a credential refbox does
not have. Do not read that as refbox having become careful on your behalf; it is the one call whose
whole purpose is to ask "is my token good?", and asking that with no token was meaningless.

schedule-processor checks for a token first and stops (`schedule-processor/src/main.rs:515`), but
that guard is not part of the refbox nine.

So your site will see unauthenticated requests arriving on three of the four endpoints the inventory
marks `bearer`, and it must treat them as failures rather than serve them. Serving an unauthenticated
call 5 hands your privileged schedule to anyone who asks, which — as the walkthrough on 2026-08-11
showed — is enough for refbox to fill its court and game pickers from data it was never authorised
to have.

**The four calls marked `none` — event list (3), event teams (4), referees (6), and team roster
(9) — never carry an `Authorization` header, in any state.** Unlike the bearer calls above, which
route through `authenticated_request` and omit the header only when refbox holds no token, these
four are built directly from the bare client and never pass through that function at all: event
list (`uwh-common/src/uwhportal/mod.rs:548-556`), event teams (`mod.rs:507`), referees (`mod.rs:459`),
team roster (`mod.rs:677-681`). This holds even when refbox is fully linked and holding a valid
token — the header is unconditionally absent on these four paths, not merely absent because there
was nothing to send. A site may safely reject any request to these four paths that arrives carrying
an `Authorization` header: refbox is never the one sending it, so refusing it costs nothing. Team
roster's path (`/api/admin/get-event-team`) is the one most likely to tempt an implementer into
requiring a token anyway, on the strength of its `/admin/` segment — see the note above under
[Full inventory](#full-inventory). Doing so does not fail loudly: refbox has no way to supply a
header this call will never carry, so requiring one here reproduces exactly the silent failure
call 9's own entry describes — the player-number grid quietly stops working, with nothing on
screen or in the log to explain why.

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

**IDs inside response bodies are always the long form, and that is enforced.** The two rules above
govern paths and query parameters — the things refbox *sends*. Every ID your site *returns* in a
JSON body (`Schedule.eventId`, `ScheduledTeam.teamId`, and the `id` of each entry in the event
list) goes through a custom deserialiser that checks it: `schedule.rs:690` for event IDs,
`schedule.rs:738` for team IDs.

It checks exactly two things:

- The value starts with exactly `events/` or `teams/`. The comparison is case-sensitive, so
  `Events/1234-A` is rejected, and a bare short form like `1234-A` is rejected in a body.
- At least three characters follow the prefix. `events/1234-A` and `events/ABC` are accepted;
  `events/AB` and `events/A` are not.

Beyond those two checks the value is opaque: it is never split on the hyphen and never matched
against a pattern, so `events/my-own-identifier` is perfectly valid. The `1234-A` shape used
throughout these examples is the Portal's convention, not a requirement your site has to imitate.

Getting this wrong fails hard, and the error message will lie to you. A rejected ID is a
deserialisation failure that discards the **entire** response, not just the offending entry — so
one malformed ID in an event list costs you the whole list. And the message is always `Invalid
format for full_id. It should start with 'events/'`, even when the prefix was perfectly fine and
the length was the real problem. If you see that error against an ID that plainly starts with
`events/`, count the characters after the slash.

Worked example, using the event and teams from the schedule example below:
- Short form, in a path: `GET /api/events/{eventId}/schedule/privileged` with `eventId` = `1234-A`
- Long form, in a query parameter: `POST /api/admin/events/stats?eventId=events/1234-A&gameNumber=1`

**The long form shown above is the value's logical shape, not the literal bytes on the wire.**
refbox builds every long-form-in-a-query-parameter value — this one, team roster's `teamId`, and
game referees' `eventId` — through reqwest's `.query()` helper, which percent-encodes it before
sending (`uwh-common/src/uwhportal/mod.rs:680`, `:334`, `:782`). So a `teamId` of `teams/5678-B`
does not arrive as `teamId=teams/5678-B`; it arrives as `teamId=teams%2F5678-B`, with the slash
replaced by its percent-encoded form. Hex case is not guaranteed either: a live capture had refbox
send the uppercase `%2F` for the same slash that `curl --data-urlencode` encoded as lowercase
`%2f` for an equivalent request. A hand-written server that reads the request line straight off
the socket — an approach this document assumes elsewhere is a live option — cannot compare the raw
query bytes against `teams/` or `events/`; it has to percent-decode the value first and match hex
case-insensitively, or every long-form ID lookup will 404.

Across the full eighteen-call inventory, exactly three calls put an ID in a query parameter, and
so are the only three that use the long form:
- Push stats — `eventId` (`uwh-common/src/uwhportal/mod.rs:334`) — one of the refbox nine, call 8
  above.
- Team roster fetch — `teamId` (`mod.rs:676`) — one of the refbox nine, call 9 above; also used by
  schedule-processor and the overlay.
- Game referees fetch — `eventId` (`mod.rs:779`) — part of the other nine.

Every other ID in this API — including every `{eventId}` in the path tables above — is the short
form.

### The schedule payload

This is the body returned by call 5 (`GET /api/events/{eventId}/schedule/privileged`). The whole
shape is deserialised as one Rust structure, so every field described below as "required" must be
present in the response — as an empty array or object where that's all there is — or the whole
schedule fails to load. Fields described as "optional" may be left out of the JSON entirely.

#### Top level: `Schedule` (`uwh-common/src/uwhportal/schedule.rs:513`)

**Five of these fields have no shape documented here, deliberately.** `nonGameEntries` and `groups`
are required but an empty array satisfies them; `standingsOrder`, `finalResultsOrder` and
`refereesByGameNumber` may be omitted entirely. They carry Portal-internal features — standings
tables, ceremonies, referee rosters by game — that a stand-in site running a tournament does not
need, and refbox does not use them to run a game. Send `[]` for the two required ones, omit the
other three, and you lose nothing. **This is a limit of this document, not an oversight:** if you
ever want those features, their shapes are not described anywhere here and you would have to ask.

| Field | Required? | Contents |
|---|---|---|
| `eventId` | required | The event ID, long form (`events/1234-A`) |
| `games` | required (may be `{}`) | **An object**, not an array — keys are game numbers as strings, values are `Game` objects (see below). The key string and the `Game`'s own `number` field should match. |
| `nonGameEntries` | required (may be `[]`) | Calendar entries (breaks, ceremonies) that aren't games. Not needed to run a game — a stub can always send `[]`. |
| `groups` | required (may be `[]`) | Pool/division structure and standings rules. Not needed to run a game — a stub can always send `[]`. |
| `timingRules` | required | Array of `TimingRule` objects (see below). Every game's `timingRule.name` must match one of these by name. **A name that matches nothing is not a parse failure and produces no error** — refbox simply runs that game on whatever timing configuration it already had loaded, silently. A typo here costs the right period lengths at a real game, and says nothing to the operator. |
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

**A `teamId` here only becomes a team *name* if call 4 returned that same ID.** This is the one
place in the contract where two separate calls have to agree with each other, and there is no
error path when they don't: refbox looks the ID up in the map it built from call 4's `teams`
array, and simply falls back if it misses. What the operator sees tells you which half is wrong:

- **The raw ID** (`teams/5678-B`) where a name should be — call 4 succeeded, but its `teams`
  array did not contain this ID. Your schedule and your team list disagree.
- **`Unknown`** — call 4 never returned successfully for this event, so there is no map to look
  in at all. Call 4's own failure path is silent (see its "On failure" above), so this may be the
  only visible symptom you get.

Neither case logs an error, and the game stays selectable and playable either way — so an ID
mismatch between your schedule and your team list is easy to ship without noticing.

#### `RefereeAssignment` (`schedule.rs:210`)

| JSON field | Required? | Contents |
|---|---|---|
| `role` | required | Free-text role name, e.g. `"Head Referee"` — refbox doesn't validate this against a fixed list |
| `userId` | optional | Portal user ID, matched against call 6's response to show a name. **Opaque and unvalidated** — unlike event and team IDs it is a plain string with no prefix rule and no length rule, so any value parses. A value that matches nothing in call 6 just means no name is shown. |
| `teamId` | optional | Team ID, **long form** — used when an official is assigned by team rather than by person, in which case `userId` is absent. It is validated exactly like any other team ID, so a malformed one fails the entire schedule parse, not just this assignment. If it can't be resolved to a team name, the raw ID is displayed. |

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
| `playerCapNumber` | integer (`foul`: integer or `null`) | The player's cap number. `null` on a `foul` only, for a team-level infraction ("both at fault") with no specific player. **On a `goal` or `penalty`, `0` means "the operator did not record a number"** — see the warning below. |
| `side` | string (`foul`: string or `null`) | `"dark"` or `"light"` — same black/white convention as push-scores. `null` on a `foul` only, alongside a `null` `playerCapNumber`. |
| `gamePeriod` | string | refbox's internal period name — one of `BetweenGames`, `FirstHalf`, `HalfTime`, `SecondHalf`, `PreOvertime`, `OvertimeFirstHalf`, `OvertimeHalfTime`, `OvertimeSecondHalf`, `PreSuddenDeath`, `SuddenDeath` (`uwh-common/src/game_snapshot.rs:129-141`) |
| `periodTime` | number (seconds, may have a fractional part) | The game clock's value the instant the event was recorded. During a timed half (regulation or overtime) this counts **down** — time remaining in the period. During Sudden Death, which has no fixed length, the clock instead counts **up** from zero — so `periodTime` there is time *elapsed*, not remaining (`refbox/src/tournament_manager/mod.rs:1855-1868`, `:2182-2188`). |
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

> **Do not treat `playerCapNumber: 0` as a real player.** On a goal or a penalty, refbox's
> cap-number keypad starts at `0` and its confirm button is available at that default, so an operator
> who records a goal without entering a number produces a record with `playerCapNumber: 0`. It is
> indistinguishable on the wire from a genuine cap number 0, and refbox itself logs it as "player
> #0". Observed live on 2026-08-10. Fouls are the exception: there the field is genuinely nullable,
> so an unattributed foul arrives as `null` rather than `0`. If you are attributing statistics to
> players, treat `0` on a goal or penalty as "unattributed" rather than creating a player for it.

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

## The other nine

**None of the nine calls in this section are needed to run a game.** Eight of them belong to
`schedule-processor`, the command-line tool a tournament admin runs before the event — to upload
the schedule, resolve coin tosses (the coin toss UWH uses to break a tie or decide a seeding when
teams finish level), and generate printed scoresheets. The ninth belongs to the stream overlay,
and only matters to a site that wants to serve the overlay's on-screen graphics. If you only want
something for the refbox to talk to during a game, you're done — [The refbox nine](#the-refbox-nine)
is the whole contract you need, and you can skip the rest of this section entirely.

Full detail on all nine below, using the same headings as the nine above so all eighteen entries
in this document can be skimmed the same way. Two things already established still apply here:

- One of these nine puts an ID in a query parameter and so uses the long form described under
  [the two ID forms](#the-two-id-forms): game referees (call 4 below).
- "Fields [caller] actually reads" lists only what the deserialising code actually pulls out of
  the response — a stand-in site can return an object with only those fields (plus whatever the
  shape requires just to parse) and the real program will work correctly.

#### 1. Log in with email and password

`POST /api/authentication`  ·  source: `uwh-common/src/uwhportal/mod.rs:264`

**When schedule-processor calls it:** schedule-processor doesn't log in up front. Each of its
privileged menu actions — Upload Schedule, Resolve Coin Tosses, and Generate Score Sheets (twice:
once for an optional "show portal display names" step, and again if a call partway through
Generate Score Sheets comes back saying it needs authentication) — checks whether it already has
a token and only prompts for a login here if it doesn't. Once one action logs in successfully,
every later action in the same run reuses that token, until a push fails and clears it (see calls
8 and 9 below). Credentials are always typed interactively — an email prompt, then a masked
password prompt — there is no command-line flag or environment variable for them.

**Authentication:** none

**Query parameters:** none

**Request body:** `{"email": "<string>", "password": "<string>"}`

**Successful response:** `200` with `{"accessToken": "<token>"}`. Note the field name:
`accessToken`, not the `accessKey` that the refbox login (call 1 of the refbox nine) returns —
these are two unrelated login flows that happen to both hand back a bearer token, under
differently-named fields.

**Fields schedule-processor actually reads:** `accessToken` on success. Nothing else in the
response is read.

**On failure:** Any non-`200` response: logged, and schedule-processor returns to its main menu
without exiting — the operator can just try the action again. The exception is the optional
"show portal display names" login: a failure there just means generation proceeds without those
names, rather than sending the operator back to the menu. There's no equivalent of refbox's two
specific `400` reasons; every failure here is treated the same way.

#### 2. Public event schedule

`GET /api/events/{eventId}/schedule`  ·  source: `uwh-common/src/uwhportal/mod.rs:647`

**When schedule-processor calls it:** When generating scoresheets, if schedule-processor doesn't
already have a login token — this is the unauthenticated alternative to the privileged schedule
call (call 5 of the refbox nine, `/schedule/privileged`). A comment on this call in the source
warns that for some events the public endpoint returns `games` as a plain JSON array rather than
the object shape the parser expects; when that happens this call fails to parse, and
schedule-processor falls back to logging in and using the privileged schedule instead.

**Authentication:** none

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with the same shape documented under
[the schedule payload](#the-schedule-payload) for the privileged call — this endpoint and the
privileged one return the same shape, just gated differently.

**Fields schedule-processor actually reads:** Same as the privileged schedule — see Data
formats; the whole shape is deserialised.

**On failure:** Any non-`200` response, or a body that doesn't parse (including the
array-vs-object mismatch above): logged, then schedule-processor automatically prompts for a
login and retries with the privileged call instead. If that also fails, the action ends there.

#### 3. Event referee name map

`GET /api/events/{eventId}/participants`  ·  source: `uwh-common/src/uwhportal/mod.rs:726`

**When schedule-processor calls it:** Every time it generates scoresheets, to attach display
names to officials — the same purpose as call 6 of the refbox nine, but reading a different
endpoint.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with participant entries, accepted in any of three shapes: a bare
JSON array, `{"participants": [...]}`, or `{"items": [...]}`. Example:
```json
{ "participants": [ { "user": { "id": "user-abc123", "username": "reef_ref" }, "rosterName": "Casey" } ] }
```

**Fields schedule-processor actually reads:** Per entry, an ID (`user.id`, falling back to
`userId`, falling back to `id`) and a display name (`rosterName` if non-empty, otherwise
`user.name`, otherwise `user.username`). An entry missing either is skipped.

**On failure:** Any non-`200` response: logged at debug level and skipped — scoresheet generation
proceeds without adding any names from this call.

#### 4. Game referee name map

`GET /api/admin/events/game-referees`  ·  source: `uwh-common/src/uwhportal/mod.rs:772`

**When schedule-processor calls it:** While generating scoresheets, at most once per game, the
first time that game's officials need a name looked up.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** `eventId` — the event ID, **long form** (`events/1234-A`), per the
query-parameter rule under [the two ID forms](#the-two-id-forms) — and `gameNumber` (plain
string, e.g. `"3"`).

**Request body:** none

**Successful response:** `200` with referee entries, accepted either as `{"referees": [...]}` or
a bare array. Example:
```json
{ "referees": [ { "user": { "id": "user-abc123", "name": "Casey Referee", "username": "reef_ref" } } ] }
```

**Fields schedule-processor actually reads:** Per entry, an ID (`user.id`, falling back to
`userId`, falling back to `id`) and a display name (`user.name` if present, otherwise
`user.username`, otherwise `rosterName`). An entry missing either is skipped.

**On failure:** Any non-`200` response: silently skipped, with no log message. The scoresheet
falls back to showing the raw user ID's suffix in place of a name for that game's officials.

#### 5. Get coin flips

`GET /api/events/{eventSlug}/schedule/coin-flips`  ·  source: `uwh-common/src/uwhportal/mod.rs:694`

**When schedule-processor calls it:** When the operator picks "Resolve Coin Tosses" from the
menu, after logging in if needed.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with `groups` and `games`, each a list of coin-flip records. A
record not yet decided has no `result`; one that's been called has a `result` naming which team
it favoured. Every field on this response accepts either `PascalCase` or `camelCase` — the
portal may send either. Example:
```json
{
  "groups": [],
  "games": [
    {
      "identifier": "1",
      "tiedTeams": [
        { "teamId": "teams/1234-A" },
        { "teamId": "teams/5678-B" }
      ],
      "result": null
    }
  ]
}
```

**Fields schedule-processor actually reads:** Everything shown above. Per tied-team entry,
exactly one of `teamId` (long form) or `pendingAssignmentName` is expected to be populated —
whichever is present is used to label the choice presented to the operator. A `result`, when
present, names which of the tied teams or groups won the toss.

**On failure:** Any non-`200` response, or a body that doesn't match this shape: logged, and
schedule-processor returns to its main menu — no retry.

#### 6. Set coin flip result

`POST /api/events/{eventSlug}/schedule/coin-flips`  ·  source: `uwh-common/src/uwhportal/mod.rs:816`

**When schedule-processor calls it:** Immediately after the operator picks a tied game (or group)
and a winning team from the menu populated by call 5.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** `force` (boolean, `true` or `false`) — set when the operator confirms
overwriting an already-decided result; ordinarily `false`. Always present, as the literal `true`
or `false` (`uwh-common/src/uwhportal/mod.rs:827`) — unlike the schedule upload in call 7 below.

**Request body:** identifies which toss is being recorded and its outcome. Unlike every other
JSON body in this document, the field names here are **`PascalCase` only** — there is no
`camelCase` form accepted, since this shape is only ever sent, never received. Example:
```json
{
  "GroupIdentifier": null,
  "CoinFlipIdentifier": "1",
  "TeamIdOrPendingAssignmentName": "teams/1234-A",
  "Kind": "Favor"
}
```
`GroupIdentifier` is `null` for a tied-game toss (as opposed to a group-seeding toss).
`TeamIdOrPendingAssignmentName` carries whichever of the two the chosen team had — the long-form
team ID once a team is resolved, or its placeholder name otherwise.

**Successful response:** `200`. The body is never parsed — only the status code matters.

**Fields schedule-processor actually reads:** none.

**On failure:** Any non-`200` response: logged, but schedule-processor doesn't return to the menu
or clear anything — the operator sees the error and can retry the same choice.

#### 7. Push schedule

`POST /api/events/{eventSlug}/schedule`  ·  source: `uwh-common/src/uwhportal/mod.rs:580`

**When schedule-processor calls it:** When the operator picks "Upload Schedule" and confirms,
after loading a schedule from a local CSV file and logging in if needed.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** `force` — set only when the operator confirms overwriting a schedule the
site already has for that event. This call is the **one exception** to how `force` is sent
everywhere else in this document: it appears as `force=true` only when forcing, and is **omitted
from the query string entirely** otherwise (`uwh-common/src/uwhportal/mod.rs:592`). A site that
requires the parameter to be present will reject every ordinary schedule upload.

**Request body:** the schedule to upload — the same shape as
[the schedule payload](#the-schedule-payload) documented for the privileged response, with two
differences: `eventId` is left out (the event is already identified by the slug in the URL), and
`refereesByGameNumber` is left out entirely. `games` is sent as a plain JSON array of `Game`
objects, not an object keyed by game number.

**Successful response:** `200`. The body is never parsed — only the status code matters.

**Fields schedule-processor actually reads:** none.

**On failure:** Any non-`200` response: logged, and schedule-processor clears its saved login
token — forcing a fresh login on the next attempt — before returning to the main menu.

#### 8. Push team map

`POST /api/events/{eventSlug}/schedule/map-teams`  ·  source: `uwh-common/src/uwhportal/mod.rs:614`

**When schedule-processor calls it:** Immediately after call 7 succeeds, in the same "Upload
Schedule" action — the two are always sent as a pair.

**Authentication:** `Authorization: Bearer <token>`

**Query parameters:** none

**Request body:** a flat JSON object mapping each placeholder team name used in the uploaded
schedule to the real team's ID, long form. Example:
```json
{ "Pool A Winner": "teams/1234-A", "Pool B Runner-up": "teams/5678-B" }
```

**Successful response:** `200`. The body is never parsed — only the status code matters.

**Fields schedule-processor actually reads:** none.

**On failure:** Same as call 7: logged, saved login token cleared, back to the main menu.

#### 9. Overlay attachments

`GET /api/admin/events/{eventId}/overlay-attachments`  ·  source: `overlay/src/network.rs:174`

**When the overlay calls it:** Once per event, the first time the overlay sees a snapshot naming
that event, and again whenever the linked event changes — fetching the event logo and sponsor
images shown on the video overlay.

**Authentication:** none

**Query parameters:** none

**Request body:** none

**Successful response:** `200` with an `overlayAttachments` array. Example:
```json
{
  "overlayAttachments": [
    { "type": "Overlay", "url": "https://example.com/event-logo.png" },
    { "type": "Sponsor", "url": "https://example.com/sponsor.png" }
  ]
}
```

**Fields the overlay actually reads:** Per entry, `type` (only `"Overlay"` and `"Sponsor"` are
recognised — anything else is ignored) and `url`. The overlay then makes a second, separate
request to that `url`, expecting raw image bytes back. A missing `overlayAttachments` array, or
one with no recognised `type`, just means no logo or sponsor image is shown.

**On failure:** Unlike every other call in this document, the overlay never checks the response's
status code — it always tries to parse the body as JSON regardless. A body that isn't valid JSON
makes that attempt panic (`overlay/src/network.rs:171-184`). Because this runs in its own
background task, the panic doesn't crash the overlay itself, but no further attempt is made for
that event unless the linked event changes again. A stand-in site should always return a JSON
body on this path, even `{}`, to avoid triggering this.

#### The overlay's other calls (same paths, different code)

The overlay does not use the shared portal client in `uwh-common` that every other call in this
document goes through — it has its own, separate networking code in `overlay/src/network.rs`,
and that code makes three more calls beyond the one above. Each hits a path already documented
elsewhere in this file, but a site that only implements the behaviour expected by the *other*
caller of that path will miss the overlay's own requests to it if it isn't watching for them:

| Path | Source | Also documented as | Auth the overlay sends |
|---|---|---|---|
| `GET /api/admin/get-event-team` | `overlay/src/network.rs:96` | Call 9 of [the refbox nine](#the-refbox-nine) (team roster) | none |
| `GET /api/admin/events/game-referees` | `overlay/src/network.rs:240` | Call 4 above (game referee name map) | none |
| `GET /api/events/{eventId}/schedule` | `overlay/src/network.rs:320` | Call 2 above (public event schedule) | none |

The first and third rows behave the same for the overlay as for schedule-processor: neither call
ever needs a token. **The middle row does not** — schedule-processor's call to the same path
(call 4 above) sends a bearer token, but the overlay's own call to that identical path never
attaches one (no `Authorization` header is set anywhere in `overlay/src/network.rs`). A site that
requires a bearer token on `/api/admin/events/game-referees` because schedule-processor sends one
will reject the overlay's request to the exact same path.

The overlay reads different fields out of these three responses than schedule-processor does,
because it's serving on-screen graphics rather than scoresheets:

- **Team roster** (`:96`): reads a top-level `name` (the team's display name, falling back to the
  literal `BLACK`/`WHITE` colour label if missing) and `logoUrl` (fetched as an image, for the
  team's flag graphic). Per roster entry it reads `rosterName` (falling back to the literal text
  `"Player"`), `capNumber`, one entry from `roles` that isn't the literal string `"Player"` (kept
  as free text — unlike call 9 of [the refbox nine](#the-refbox-nine), there is no filtering: every
  roster entry is included here, not just Players, Captains, and Vice-Captains), and two photo
  URLs under `photos` (`uniform`,
  and `darkGear` or `lightGear` depending on which side the team is on), each fetched separately
  as an image.
- **Game referees** (`:240`): reads `referees[].role`, mapped to one of three on-screen labels —
  `"Water1"` / `"Water2"` / `"Water3"` all become "Water", `"Chief"` becomes "Chief",
  `"TimeOrScoreKeeper"` becomes "Timekeeper"; any other value is dropped with a logged warning.
  Also reads `referees[].user.name` and a photo URL under `referees[].user.photos` (`uniform` and
  `inGear`). An entry missing `user.name` is dropped.
- **Public schedule** (`:320`): expects a shape that diverges from
  [the schedule payload](#the-schedule-payload) documented above in two ways. It reads a
  top-level `court` and `startsOn` directly off the whole response body, not per-game. And it
  treats `games` as a plain JSON **array** to search through by matching `number`, not the
  object-keyed-by-game-number shape the schedule calls document elsewhere — the same
  array-vs-object ambiguity flagged under call 2's own description above. Per matching game it
  then reads `dark.assignment.teamId` / `light.assignment.teamId` — note the extra `.assignment`
  nesting, compared to the `dark.teamId` / `light.teamId` shape used everywhere else in this
  document. A site serving both the overlay and schedule-processor/refbox from the same schedule
  response needs to satisfy both shapes at once, or one of the callers won't find what it needs.

The overlay's failure handling for these three calls also differs, call by call, from every other
call in this document — none of them check the response status code before deciding what to do
with the body:

- **Team roster** (`:96`): same as call 9 above — a connection failure, or a body that isn't
  valid JSON, panics the background task fetching that team's information
  (`overlay/src/network.rs:94-105`). The panic doesn't crash the overlay, but that game's team
  shows no roster, photos, or flag until a later game or event change triggers a fresh fetch.
- **Game referees** (`:240`): the only one of the three handled gracefully — a connection failure
  or an unparseable body is caught and logged as a warning, and the game simply shows no referee
  information (`overlay/src/network.rs:239-245`).
- **Public schedule** (`:320`): a connection failure is retried automatically every 5 seconds,
  indefinitely, until it succeeds (`overlay/src/network.rs:317-423`) — the most forgiving failure
  handling of any call in this document. Once a response does arrive, though, a body that isn't
  valid JSON, or one where the requested game number can't be found in it, ends that attempt with
  a logged error and no further retry — the overlay only tries again the next time refbox reports
  a different game or event.

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
