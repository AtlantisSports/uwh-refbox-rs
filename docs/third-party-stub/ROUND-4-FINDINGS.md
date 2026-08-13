# Sealed-room round 4 — findings

**Run:** 2026-08-13, against the document as of commit `753e355a` (rebased onto master, citations
audited, team roster promoted to call 9).

**Method:** a fresh agent in a directory containing only a copy of `third-party-integration.md`,
forbidden from opening any Rust source, any existing stub, or any git tree. It built
`stub_site.py` from the document alone and exercised every call it implemented.

**What the round confirmed about this branch's own changes:**

- It derived **nine** must-answer calls from the document unaided — it was never told a number.
  The promotion of the team roster to call 9 reads clearly.
- It implemented the `roles` filter as an **inclusion** test, and deliberately seeded its fixture
  with a playing coach, a coach-only member, and cap numbers `0` and `100` to exercise both
  documented filters. The corrected wording survives a blind read.

**Standing caveat from the reviewer:** there is no refbox in the sealed room. "Works" means
"matches what the document says", not "a refbox accepted it". Findings 1, 4, 5, 8, 12 and 23 are
the class only a live refbox can settle.

**Status key:** FIXED · OPEN · CODE? (may be a software defect, not a documentation one)

---

## A. Contradictions — the document disagreeing with itself

### 1. Call 8's request body is described two ways — SEVERE — OPEN

Call 8's own entry says "**Request body:** A JSON object of per-team, per-player statistics", and
calls it "the other large, shared **response** shape". Data formats says "The body is **a bare JSON
array** of event objects, with no wrapping object", and the worked example is an array.

An implementer who reads only the call entry writes a dict-shaped model and rejects or 500s on an
array — *after* the score push has already succeeded. The item goes stats-pending, is not
auto-retried, and the indicator never reddens. The quietest failure mode the document describes.

### 2. The `force`-is-omitted cross-reference names the wrong call — SEVERE — OPEN

The push-scores entry points at "the coin-flips upload … (inventory #17)" as the call that behaves
differently. But the coin-flips upload entry says it **always** sends `force`, "unlike the schedule
upload", and the schedule-upload entry calls itself "the one exception". Followed literally, an
implementer makes `force` mandatory on schedule upload — which the document itself says "will
reject every ordinary schedule upload".

### 3. The public schedule must return two mutually exclusive shapes — SEVERE — CODE?

The public-schedule entry says it returns "the same shape" as the privileged one (`games` as an
object keyed by number, `dark.teamId`). The overlay section says the overlay, on that same path,
"treats `games` as a plain JSON array", reads a top-level `court` and `startsOn`, and reads
`dark.assignment.teamId`. The document then instructs a site to "satisfy both shapes at once".

`games` cannot be both an object and an array. Not achievable as written, and the document never
says which caller to sacrifice. **This may be a genuine defect in the overlay rather than in the
prose** — resolving it needs a decision, and possibly a code change on its own branch.

### 4. Three cross-references inside "the other nine" pointed at themselves — FIXED (`6507438a`)

Introduced by this branch's renumbering: entry 6 cited "call 6" and entry 8 cited "call 8" twice.
Corrected to call 5 and call 7. The earlier sweep searched for "call N above/below" and these carry
no direction word.

The reviewer's wider point stands: a document that warns twice about its two numbering schemes and
then misnumbers its own references teaches a reader to distrust every "call N" in it.

---

## B. Silences and ambiguities that change what a site does

### 5. No contract at all for the admin half of the link handshake — SEVERE — OPEN

The document explicitly declines to specify it. But without a pending-link registry, call 1 can
never succeed, so the tester had to invent the registration endpoint, a six-digit code, a 10-minute
TTL, and single-use consumption — all four are theirs, not the document's.

Most likely failure: skipping the registry entirely. The document *tells* you refbox cannot detect
the shortcut, then gives no shape for the honest version. A constant token takes ten minutes and
passes every test the document describes.

**No sealed-room round can ever test this**, because the tester must invent the missing half.

### 6. Nothing throttles guesses at the link code — SEVERE (security) — OPEN

The document enumerates what implementing call 1 means — record, bind, reject with the two `reason`
strings, expire, single-use, revocable — and says nothing about failed attempts. The code space is
at most 900,000, the `refBoxId` is on the refbox screen, and each wrong guess just returns
`InvalidCode`. A diligent implementer following the list ships an unthrottled oracle believing the
security section satisfied.

### 7. `filter=Past` is undefined — exclusive or additive? — SEVERE — OPEN

The parameter is described as "from an operator setting for whether to include past events", which
describes a checkbox (past **and** current), while the value name reads as an exclusive enum. The
site applies it and refbox displays exactly what comes back. Guess wrong and an operator ticking
"include past events" mid-tournament watches their live event vanish from the picker.

### 8. Whether a tokenless call 2 must be refused, in the build you can download — SEVERE — OPEN

The rules section says refbox makes three of the four bearer calls tokenless and that call 2 is the
exception — but that narrowing lives in the **unreleased** custom-source work. The only route
available today is the environment override, where it does not exist, so a released refbox does
send verify with no token.

The reviewer's summary: **"The document's single most important warning is protected by a guard
that does not exist in the build a stranger can download."** An implementer who reads the rules as
current fact, and who has absorbed the permissive tone of the score-push advice, answers `200` —
reproducing the documented disaster of a green token row over an unlinked refbox.

### 9. `roles` and `capNumber` are never declared required or optional — SEVERE — OPEN

Call 9 describes two filters but never the required field set; it gives a *sufficient* set
("`capNumber` and `roles` alone"), not a necessary one. Unanswered: an entry with no `roles`, no
`capNumber`, or `"capNumber": "7"` as a string. Omitting `roles` for an all-players team, or string
cap numbers from a CSV import, discards the whole roster — the document's own named silent failure.

### 10. TLS certificate requirements are never stated — SEVERE for a real deployment — OPEN

Plain `http` and the typed scheme are covered; self-signed certificates, private CAs, hostname
verification and pinning are not. A security-minded implementer puts the stand-in on `https://`
with a self-signed cert — the obvious pool-LAN choice — and if refbox rejects it, the document says
the failure is indistinguishable from unreachable. No documented diagnosis, no documented fix.

### 11. Query-parameter encoding is never specified — MODERATE — OPEN

Every example shows `teamId=teams/5678-B` with a raw slash; nothing says whether refbox
percent-encodes it. The document's own recommended validation route is a hand-written server, and a
hand-rolled query parser that splits on `&`/`=` without unquoting 404s every roster fetch. Silent
loss of the player-number grid.

### 12. Nothing identifies which game mode a request comes from — MODERATE — OPEN

No header, query parameter or path segment carries hockey-vs-rugby, so a stand-in cannot tell UWH
from UWR even if it wanted to, and the fifteen `TimingRule` fields are never discussed for rugby.

### 13. The document's own worked examples are mutually inconsistent — MODERATE — OPEN

Call 4's example lists only two teams; the schedule example uses a third for game 2's dark team,
while claiming the examples "describe one consistent tournament". The document elsewhere explains
that a `teamId` missing from call 4 renders as a raw ID with nothing logged. Copying both examples
verbatim — the single most likely thing a stranger does — ships the exact defect the document
describes.

### 14. Redirects are undefined behaviour, colliding with "exactly 200 counts as success" — MODERATE — OPEN

Redirects are "whatever the underlying HTTP client does by default … do not build a site that
depends on a particular behaviour", yet only `200` is success. An nginx http→https redirect or
trailing-slash canonicalisation is the default posture of every hosting stack; if the client does
not follow, every call fails, including score pushes that then queue for 120 hours. "Do not depend
on a particular behaviour" is not actionable when the two behaviours are "works" and "loses every
result".

### 15. HTTP version and connection handling have no stated requirement — MINOR — OPEN

What refbox *sends* is documented; what it will *accept* is not — HTTP/1.0 with `Connection:
close`, chunked responses, gzip. Python's `http.server` **defaults to HTTP/1.0 and closes after
each response**; with ~40 concurrent roster GETs plus the teams and schedule burst, that is 40+
handshakes against a 10-second budget, and nothing tells you to change it.

### 16. Whether the "auth: none" calls may still carry an `Authorization` header — MINOR — OPEN

Documented for calls 5/7/8; silent for 3/4/6/9. A strict site would reject a header that rides
along — or, the other direction, would *require* one on `get-event-team` because the path says
`/admin/`, which the document warns about.

### 17. Which value lands in the score-push path: the `games` key or `Game.number`? — MINOR — OPEN

The document says the key and `number` "should" match — not must. A site whose display number
differs from its internal key files results under a game that does not exist, silently, because the
document rightly forbids rejecting.

### 18. Stats idempotency is never addressed — MINOR — OPEN

Retry and RETRY ALL make duplicate pushes routine; nothing says replace vs append. Appending
double-counts every retried game, invisible until someone reads a statistics page.

### 19. No guidance on which 100 events to return — MINOR — OPEN

The site applies `limit`, refbox never paginates, the remainder are "simply unreachable".
Truncating in database order can put the operator's current event outside the first 100, and
nothing reports it.

### 20. An unauthenticated score push has no documented resolution — MINOR — OPEN

The queue retries for 120 hours, but nothing says a queued item picks up a token acquired later. If
it does not, the first game of every tournament on the in-app route is lost, and the document as
written would not reveal it.

### 21. `204` is a failure — but is a zero-length `200` safe? — MINOR — OPEN

Verify's entry says "the body can be empty"; the rules section says "Return `200` with a body — an
empty JSON object is fine". The softer statement comes first, in the call entry.

### 22. Timestamp offsets other than `Z` are never ruled in or out — MINOR — OPEN

What refbox *writes* is documented; whether `2026-08-08T09:00:00+02:00` parses on `startsOn` is
not, and the format names imply strictness. A site serving local-time offsets could lose the entire
schedule, and this failure family reports itself misleadingly.

### 23. How refbox decides which game is "upcoming" is never stated — MINOR — OPEN

Call 9 refires "for the two teams of the upcoming game". By `startsOn`? By number? Restricted to
the selected court? A schedule with duplicate start times or non-monotonic numbers behaves
unpredictably, with no rule to test against.

### 24. Status codes carry no meaning, so the site cannot signal anything — MINOR — OPEN

`404`/`400`/`500` are indistinguishable. Stated plainly: an event deleted on the site is presented
to the operator identically to a site that is down.

### 25. Revoked-token behaviour is asserted but not specified — MINOR — OPEN

The document demands revocability but never says what a revoked key returns or how the operator
recovers. An HTTP response prompts re-login while a dropped connection does not — so a site that
revokes by closing the socket strands the operator with a red indicator and no route back.

### 26. `slug` "must be present to parse" — is `""` acceptable? — MINOR — OPEN

"Present" is not "non-empty"; being wrong costs the whole event list.

### 27. Call 6's minimal valid body is inferred, not stated — MINOR — OPEN

Three separate optionality statements from which `{}` *should* follow, never stated directly.

### 28. Sizing guidance covers call 9 but not call 4 — MINOR — OPEN

~40 concurrent roster GETs are warned about; teams are fetched for **every listed event** — up to
100 more concurrent requests with `limit=100` — unmentioned.

### 29. The `timingRule` name-mismatch trap is unverifiable from the site side — MINOR — OPEN

A non-matching name runs the game "on whatever timing configuration it already had loaded",
silently. A site operator cannot distinguish "my rule applied" from "a stale one did" except by
watching a real game run with wrong period lengths.

### 30. SITE-row parsing: the base URL for the `/api/events/` branch is implied — MINOR — OPEN

Trailing slashes are addressed for the environment variable but not for the typed address, so
`https://site/api/1234-A/` presumably yields an event called `1234-A/`.

### 31. No documented way to check the site is reachable before a game — MINOR — OPEN

No ping or health path, and the operator-visible indicator reflects only call 2, which needs a
token. A site operator setting up at 8 a.m. cannot prove the site answers until someone has already
linked.

---

## C. Calls where the tester could not tell whether authentication was required

1. **Call 2 with no `Authorization` header** — see finding 8.
2. **The four "none" calls (3, 4, 6, 9) when refbox holds a token** — see finding 16.
3. **`/api/admin/events/game-referees`** — outside the nine; needs a bearer from
   schedule-processor and none from the overlay. That path cannot be given one consistent auth
   rule; the document says so and offers only "watch out".
4. **Call 1** — "auth: none" means "no bearer token", not "unauthenticated": the pending-link
   registry *is* the authentication, and its design is delegated entirely to the implementer
   (findings 5 and 6).

## D. Guesses baked into the tester's fixture

Code TTL 10 min; token = 24 hex chars; `404` unknown event/team; `401` every auth refusal;
replace-not-append for stats; sort-then-truncate for the event list; `Z`-suffixed whole-second
timestamps. Each is a place the document left the decision to the implementer.
