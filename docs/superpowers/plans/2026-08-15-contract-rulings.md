# Third-party contract — rulings of 2026-08-15

Recorded here because the SDD ledger these were taken in is gitignored; `b3588756` exists for the
same reason. This is the tracked record of what was decided, by whom, and what was built from it.

**Context:** an adversarial review of the whole branch (dumped to
`docs/audit-archive/2026-08-14-third-party-integration-adversarial-dump.md` in the primary worktree)
closed six presentation defects. What remained were the seven sealed-room round-4 findings nobody
could close without a decision or a live refbox: 3, 5, 6, 8, 10, 14 and 31.

All are now closed. **Four needed a ruling. Three did not — the source answered them.** (Finding 31
is the fourth of those, and needed nothing at all: its premise was wrong.)

---

## The four rulings

### Finding 5 — the admin half of the link handshake

**Ruled: non-normative worked example.** "This half is yours entirely" stays the rule; the document
adds one worked shape, labelled as an example and not a specification.

Options offered were: worked example (recommended), prescribe it normatively, leave the silence as
it is, or state explicitly that it is out of scope. The reasoning for the recommendation was that
without *some* shape a stranger cannot build the security-critical half at all — the sealed-room
tester had to invent a registration endpoint, a code, a 10-minute expiry and single-use consumption,
all four theirs — while prescribing it would have the document claim authority over screens refbox
never touches and cannot verify.

**Built:** a short prose flow after the obligations list. No endpoint paths, no JSON — inventing an
admin API is exactly what the ruling declined to do. The "yours entirely" bullet is unchanged.

### Finding 6 — throttling guesses at the link code

**Ruled: require it normatively.** Against the recommendation, which had been to fold it into the
finding-5 example under the document's existing "Recommendation for the site, not a refbox
guarantee" label.

The concern raised with the recommendation was consistency: mandating one part of the pairing design
while leaving the rest an example. That concern turned out to be unfounded — the document already
carries a normative bullet list ("Concretely, what implementing it means"), so throttling became
another obligation there while the finding-5 example shows a shape that satisfies them all. The
admin-half bullet inside that same list already says the shape is the implementer's.

**Built:** a new bullet in the obligations list, naming the exposure (900,000 codes, the `refBoxId`
visible on screen, free guesses) and leaving the mechanism open.

### Finding 10 — TLS certificates

**Ruled: document the OS trust-store route; no code change.**

Options offered were: the trust-store route (recommended), a refbox option pointing at a certificate
file, a blanket "accept any certificate" flag, or state the behaviour and offer nothing. The
recommendation turned on the fact that the trust-store route works today and keeps the access key
encrypted — which is the reason the https requirement was deliberately kept when the http override
was explained.

**Built:** the certificate behaviour stated, the CA route documented, and the Raspberry Pi read-only
overlay named as a real cost rather than glossed.

### Finding 3 — the overlay's schedule shape

**Ruled: find out what the Portal actually serves before deciding.** No code change yet.

**Built, and it overturned the question.** 100 events measured on 2026-08-15 across the production
and dev APIs, unauthenticated `GET`s only — **every one returned `games` as an array** with
`dark.assignment.teamId`. Zero objects. Consequences, all now in the document:

- The overlay's array reading is **correct** for that endpoint.
- The document's claim that the public and privileged endpoints "return the same shape" was
  **false**.
- `get_event_schedule_public` **cannot succeed** against the real Portal — it deserialises into
  `Schedule`, which needs an object-keyed `games` and an `eventId`, and that endpoint returns
  neither. schedule-processor's unauthenticated route always falls through to a login. That path has
  presumably never worked.
- Separately, the overlay reads `court` and `startsOn` from the **top level**, where neither key
  exists in any of the 100 responses, so both silently become empty strings.

**Still open for the human, now decidable on evidence rather than a source comment:** whether
`get_event_schedule_public` should be fixed to parse the array or removed, and whether the overlay's
top-level `court`/`startsOn` reads should be repointed at the per-game fields. Both are code changes
on their own branch.

---

## The four that needed no ruling

Listed because the pattern matters more than the answers.

- **Finding 14 (redirects)** — never undefined. Nothing configures a redirect policy, so the HTTP
  client's default applies: up to ten redirects followed, and a downgrade to plain `http` refused
  whenever TLS is required. The document's "do not depend on a particular behaviour" was *weaker
  than the truth*, which is what made the finding read as unactionable.
- **Finding 10's factual half** — certificate validation is the client's untouched default. Only the
  product question ("should refbox support self-signed?") ever needed a person.
- **Finding 8 (tokenless verify in the released build)** — had sat in the "needs a live refbox" pile
  since 2026-08-13. Settled in two commands: `v0.4.9`'s health check has no `has_token()` guard,
  `HEAD`'s does. A live run would have confirmed what the tag already showed.
- **Finding 31 (no way to check a site is reachable before a game)** — the premise was wrong, so
  there was nothing to rule on. Reachability never depended on call 2: turning Portal mode on fires
  call 3 (event list) and then call 4 (teams) for every event returned, both unauthenticated, so
  requests reach a site's log before anyone has linked. What *does* depend on call 2 is the
  operator-visible indicator, which is why a site can be up and answering and still show red. Call
  2's entry now says both things.

**The pattern: four findings escalated to the human on this branch turned out to be answerable
without one** — `filter=Past` (re-triaged during round-4 phase 1), then 14, 8 and 31 here. A fifth,
finding 10, was half answerable: only the product question ever needed a person. Before putting a
question to someone, check whether the source, a git tag, or an unauthenticated request answers it.
The cost of not checking is not just their time — it is a finding sitting open for two days under a
"blocked" label it did not deserve.

The counterweight, from the same branch: a claim that *looks* answerable from source can still be
wrong. Four claims on this branch had individually correct citations attached to sentences the code
did not support. Read the cited function and the frames above it, not just the cited line.

---

## Status after these rulings

Every round-4 finding is closed. What remains is not a finding but two ordinary code questions
raised by finding 3's evidence, plus the follow-ups recorded in
`docs/superpowers/plans/2026-08-14-third-party-doc-hardening.md`: converting the ~60 shorthand
citations to full paths, the "reference stand-in" contradiction at `:985`, and `__pycache__` missing
from `.gitignore`.
