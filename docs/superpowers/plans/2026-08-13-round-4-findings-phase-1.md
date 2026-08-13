# Round 4 Findings, Phase 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every round 4 finding that can be settled from the source code, so the contract document stops contradicting itself and stops leaving a third-party implementer to guess.

**Architecture:** Each finding is a question the document failed to answer. The work is always the same shape: establish the ground truth from the code, then write it into the document in the place the reviewer actually looked. The plan gives the exact question and the exact file to answer it from, but does not script the prose — the wording has to fit its surrounding paragraph, and a plan that dictates sentences produces text that reads like it was pasted in.

**Tech Stack:** Markdown; `rg`/`grep` over the Rust workspace; Python 3 for the existing checkers.

**Spec:** `docs/third-party-stub/ROUND-4-FINDINGS.md` — the full ledger, with severity and the reviewer's reasoning for each finding. Read it alongside this plan; the plan cites finding numbers and does not repeat their rationale.

## Global Constraints

- **Worktree:** `/home/estraily/projects/refbox-third-party-contract`, branch `docs/workspace/get-event-team-attribution`.
- **Document under edit:** `docs/third-party-integration.md`. This is the only file this plan changes, apart from the ledger's status column.
- **Do not change Rust code.** This plan documents what the software does; it does not alter it. If a finding turns out to be a software defect, mark it `CODE?` in the ledger and stop — do not fix it here.
- **Do not invent guarantees.** Where behaviour is genuinely undefined, say so and name the consequence. "The document does not specify this, and a site that guesses wrong loses X" is a legitimate and useful sentence. A fabricated promise is not.
- **Every claim gets a citation** in the document's existing style — `path/to/file.rs:LINE`. After editing, `python3 docs/third-party-stub/check_citations.py` must still report 0 unresolvable.
- **Update the ledger** as you go: change `OPEN` to `FIXED` with a one-line note. A finding closed without a ledger update is a finding that gets re-litigated next round.
- **Do not push or open the PR.** Separate approval.
- **Commit convention:** `docs(workspace): description`, lowercase, imperative, ~72 chars.

---

### Task 1: The two settled contradictions

Both are already resolved against the code — no investigation needed, just the edit. They are first because they are the only findings where the document is provably, internally wrong.

**Files:**
- Modify: `docs/third-party-integration.md`
- Modify: `docs/third-party-stub/ROUND-4-FINDINGS.md`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing later tasks depend on.

- [ ] **Step 1: Fix finding 1 — call 8's request body**

Ground truth, already established: `refbox/src/tournament_manager/game_stats.rs:96-103` is
`serde_json::to_string(&events)` over a `Vec<Event>`. The body is a **bare JSON array**.

In call 8's entry, the "Request body" line currently says "A JSON object of per-team, per-player
statistics" and calls it "the other large, shared **response** shape". Both halves are wrong: it is
an array, and it is a *request* shape. Correct both, and keep the pointer to Data formats.

- [ ] **Step 2: Fix finding 2 — the `force` cross-reference**

Ground truth: `uwh-common/src/uwhportal/mod.rs:592` wraps the parameter in `if force`, so the
**schedule upload** omits it entirely when false. `mod.rs:827` sends `.query(&[("force", force)])`
unconditionally, so the **coin-flips upload** always sends it.

The push-scores entry currently points at the coin-flips upload (inventory #17) as the call that
behaves differently. It is the schedule upload (inventory #10). Fix the reference and the
inventory number.

- [ ] **Step 3: Verify citations still resolve**

```bash
cd /home/estraily/projects/refbox-third-party-contract
python3 docs/third-party-stub/check_citations.py | tail -1
```

Expected: `69 citations checked, 0 unresolvable` (or a higher count if you added citations).

- [ ] **Step 4: Update the ledger and commit**

Mark findings 1 and 2 `FIXED` with a one-line note each.

```bash
git add docs/third-party-integration.md docs/third-party-stub/ROUND-4-FINDINGS.md
git commit -m "docs(workspace): fix two contradictions found by sealed-room round 4"
```

---

### Task 2: Roster and encoding silences (findings 9, 11)

These two share a failure mode: both silently cost the operator the player-number grid, which is
the document's own named example of a failure nobody is told about.

**Files:**
- Modify: `docs/third-party-integration.md` (call 9's entry, and the two-ID-forms section)
- Modify: `docs/third-party-stub/ROUND-4-FINDINGS.md`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing later tasks depend on.

- [ ] **Step 1: Answer finding 9 — which fields are required?**

Question: what happens to a roster entry with no `roles` key, with no `capNumber`, or with
`"capNumber": "7"` as a string?

```bash
sed -n '75,145p' uwh-common/src/uwhportal/mod.rs
```

Read `parse_roster_json`. Note specifically: `.and_then(|v| v.as_u64())` on `capNumber` — a JSON
*string* does not satisfy `as_u64`, so it yields `None`. And `member.get("roles")` returning `None`
leaves all three role flags false, which the filter then drops.

Write into call 9's entry the **required** field set, distinct from the sufficient one already
there, and state what a string cap number does.

- [ ] **Step 2: Answer finding 11 — is the query parameter encoded?**

Question: does refbox percent-encode the `/` in `teams/5678-B`?

```bash
sed -n '671,690p' uwh-common/src/uwhportal/mod.rs
```

The call uses `reqwest`'s `.query(&[("teamId", &team_id_full)])`, which percent-encodes. Confirm
against a live capture if one is available — the stub logs show `teamId=teams%2F5678-B` from refbox
and `teams%2f5678-B` from curl, i.e. **encoded, and with inconsistent hex case**.

State in the document that the value arrives percent-encoded, that hex case is not guaranteed, and
that a hand-rolled query parser must unquote. The document elsewhere encourages reading the request
off the socket, which makes this trap reachable by following its own advice.

- [ ] **Step 3: Verify, update the ledger, and commit**

```bash
python3 docs/third-party-stub/check_citations.py | tail -1
git add docs/third-party-integration.md docs/third-party-stub/ROUND-4-FINDINGS.md
git commit -m "docs(workspace): state the roster field and encoding requirements"
```

---

### Task 3: Authentication and status-code silences (findings 16, 21, 24, 25, 26, 27)

**Files:**
- Modify: `docs/third-party-integration.md` (the "Rules that apply to every call" section, and the
  individual call entries named below)
- Modify: `docs/third-party-stub/ROUND-4-FINDINGS.md`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing later tasks depend on.

- [ ] **Step 1: Finding 16 — does an `Authorization` header ride along on the "none" calls?**

```bash
grep -n "fn authenticated_request" -A12 uwh-common/src/uwhportal/mod.rs
grep -n "authenticated_request\|self.client.get\|self.client.post" uwh-common/src/uwhportal/mod.rs
```

Determine, per call, whether it goes through `authenticated_request` (attaches the header when a
token exists) or the bare client. Document the answer for calls 3, 4, 6 and 9, and say explicitly
whether a site may reject a request that carries an unexpected header.

- [ ] **Step 2: Finding 21 — is a zero-length `200` safe?**

Two statements disagree in tone: verify's entry says "the body can be empty", the rules section
says return "an empty JSON object". Establish which calls parse their response body at all:

```bash
grep -n "response.json::<" uwh-common/src/uwhportal/mod.rs
```

A call that never parses cannot care. Make the two statements agree, and state the safe answer
(`200` with `{}`) as the recommendation rather than leaving a reader to reconcile them.

- [ ] **Step 3: Findings 24 and 25 — what a site can signal, and revocation**

Finding 24 needs no code: the document already says `404`/`400`/`500` are indistinguishable. Add
the consequence the reviewer drew out — an event deleted on the site is presented to the operator
identically to a site that is down.

For finding 25, establish what happens on a `401` versus a dropped connection:

```bash
grep -n "StatusCode::UNAUTHORIZED\|is_success\|status()" uwh-common/src/uwhportal/mod.rs | head -20
```

Document what a revoked key should return and how the operator recovers, including that recovery
depends on the event picker still working (call 3 needs no token).

- [ ] **Step 4: Findings 26 and 27 — `slug` emptiness and call 6's minimal body**

```bash
grep -n "slug" uwh-common/src/uwhportal/schedule.rs | head
sed -n '449,470p' uwh-common/src/uwhportal/mod.rs
```

For `slug`, determine whether deserialisation requires the key or a non-empty value. For call 6,
state the minimal valid body directly instead of leaving it to be inferred from three separate
optionality statements.

- [ ] **Step 5: Verify, update the ledger, and commit**

```bash
python3 docs/third-party-stub/check_citations.py | tail -1
git add docs/third-party-integration.md docs/third-party-stub/ROUND-4-FINDINGS.md
git commit -m "docs(workspace): close the auth and status-code silences"
```

---

### Task 4: Schedule, timestamp and example silences (findings 13, 17, 22, 23, 29, 30)

**Files:**
- Modify: `docs/third-party-integration.md` (the schedule payload section, the worked examples, the
  SITE-row section)
- Modify: `docs/third-party-stub/ROUND-4-FINDINGS.md`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing later tasks depend on.

- [ ] **Step 1: Finding 13 — make the worked examples consistent**

The call 4 example lists two teams; the schedule example uses a third (`teams/9012-C`) for game 2's
dark team, while the document claims the examples describe one consistent tournament. Either add
that team to the call 4 example or change the schedule example. Adding it is preferable — it keeps
the schedule example's two distinct games.

Then re-read both examples end to end and confirm every ID appearing in one is present in the
other.

- [ ] **Step 2: Finding 17 — the `games` key versus `Game.number`**

```bash
grep -n "games" uwh-common/src/uwhportal/schedule.rs | head -20
```

Establish which value refbox actually puts in the score-push path. Change the document's "should
match" to a definite statement, and name the consequence of a mismatch (results filed under a game
that does not exist, silently, because the document rightly forbids rejecting).

- [ ] **Step 3: Finding 22 — timestamp offsets**

```bash
sed -n '1,30p' uwh-common/src/uwhportal/schedule.rs
```

Read the `iso8601` config. Determine whether a non-`Z` offset such as `+02:00` parses. State the
answer for `startsOn`; the document currently documents only what refbox *writes*.

- [ ] **Step 4: Findings 23, 29, 30 — upcoming game, timing-rule trap, SITE-row parsing**

```bash
grep -n "fn roster_refresh_tasks" -B12 refbox/src/app/mod.rs
grep -n "timing_rule\|timingRule" refbox/src/app/mod.rs | head
grep -n "fn parse_site\|trailing\|trim_end_matches" refbox/src/app/mod.rs | head
```

For finding 23, state how the "upcoming game" is chosen. For 29, state plainly that a site operator
cannot verify a timing rule applied. For 30, state whether a trailing slash on a typed address
yields an event ID with a slash in it.

- [ ] **Step 5: Verify, update the ledger, and commit**

```bash
python3 docs/third-party-stub/check_citations.py | tail -1
git add docs/third-party-integration.md docs/third-party-stub/ROUND-4-FINDINGS.md
git commit -m "docs(workspace): close the schedule and timestamp silences"
```

---

### Task 5: Transport, sizing and queue silences (findings 12, 15, 18, 19, 20, 28)

**Files:**
- Modify: `docs/third-party-integration.md`
- Modify: `docs/third-party-stub/ROUND-4-FINDINGS.md`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing later tasks depend on.

- [ ] **Step 1: Finding 20 — does a queued item pick up a token acquired later?**

This is the highest-value item in this task: if the answer is no, the first game of every
tournament on the in-app route is lost.

```bash
grep -n "fn.*retry\|access_token\|set_token" refbox/src/portal_manager/mod.rs | head -20
```

Establish whether the queue re-reads the token at send time or captures it at enqueue time. Document
the answer either way — including, if it re-reads, that this is what makes the in-app route's
unauthenticated first request recoverable.

- [ ] **Step 2: Findings 15 and 28 — transport and concurrency**

For finding 15, state what refbox accepts, not only what it sends. Note the practical trap the
reviewer hit: Python's `http.server` defaults to HTTP/1.0 and closes each connection, which turns
the roster burst into 40+ handshakes.

For finding 28, establish whether teams really are fetched for every listed event:

```bash
grep -n "request_event_teams\|RecvEventList" -A6 refbox/src/app/mod.rs | head -30
```

If so, add the sizing note alongside the existing call 9 warning.

- [ ] **Step 3: Findings 12, 18, 19 — mode, idempotency, event ordering**

For finding 12, confirm no request carries a mode marker and state it — a stand-in cannot tell UWH
from UWR. For 18, state whether a site should replace or append duplicate stats pushes, given that
RETRY ALL makes duplicates routine; if refbox cannot know, say the site must choose replace and why.
For 19, state which events a site should return when truncating to `limit`, and that refbox never
paginates.

- [ ] **Step 4: Verify, update the ledger, and commit**

```bash
python3 docs/third-party-stub/check_citations.py | tail -1
git add docs/third-party-integration.md docs/third-party-stub/ROUND-4-FINDINGS.md
git commit -m "docs(workspace): close the transport, sizing and queue silences"
```

---

### Task 6: Whole-document verification

**Files:**
- No edits unless a check fails.

**Interfaces:**
- Consumes: every preceding task.
- Produces: evidence the document is internally consistent after roughly twenty edits.

- [ ] **Step 1: Citations**

```bash
cd /home/estraily/projects/refbox-third-party-contract
python3 docs/third-party-stub/check_citations.py | tail -1
```

Expected: `0 unresolvable`.

- [ ] **Step 2: Internal links**

```bash
python3 - <<'EOF'
import re, pathlib
t = pathlib.Path("docs/third-party-integration.md").read_text()
heads = set()
for line in t.splitlines():
    m = re.match(r'^#+\s+(.*)$', line)
    if m:
        a = re.sub(r'[`*]', '', m.group(1).lower())
        a = re.sub(r"[^\w\s-]", '', a)
        heads.add(re.sub(r'\s+', '-', a.strip()))
links = set(re.findall(r'\]\(#([^)]+)\)', t))
bad = sorted(l for l in links if l not in heads)
print("internal links:", len(links), "| BROKEN:", bad if bad else "none")
EOF
```

Expected: `BROKEN: none`.

- [ ] **Step 3: Every "call N" reference points outside its own entry**

This is the check that would have caught finding 4, and it must be run after any renumbering:

```bash
awk '/^## The other nine/,/^## Keeping this document honest/' docs/third-party-integration.md \
  | awk '/^#### /{h=$0} /[Cc]all [0-9]/{print h" || "$0}' | grep -v "refbox nine"
```

Read every row. A row whose entry heading number matches the call number in its text is a
self-reference and a bug.

- [ ] **Step 4: Section numbering**

```bash
sed -n '/^## The refbox nine/,/^## Data formats/p' docs/third-party-integration.md | grep "^#### "
sed -n '/^## The other nine/,/^## Keeping this document honest/p' docs/third-party-integration.md | grep "^#### [0-9]"
```

Expected: 1–9 in each, no gaps or repeats.

- [ ] **Step 5: Ledger has no stale OPEN entries**

```bash
grep -c "OPEN" docs/third-party-stub/ROUND-4-FINDINGS.md
```

Every remaining `OPEN` must be one of the deferred findings listed under "Out of scope" below. Any
other `OPEN` means a task was skipped.

- [ ] **Step 6: Report and stop**

Summarise which findings are closed, which remain and why. Do not open the PR — that needs
separate approval, and phases 2 and 3 are still outstanding.

---

## Out of scope for this plan — deferred, with reasons

These stay `OPEN` in the ledger after this plan completes. They are not forgotten; they are blocked
on something this plan cannot supply.

**Needs a decision from the human (phase 2):**

- **Finding 3** — the public schedule must return `games` as both an object and an array. May be a
  defect in the overlay rather than in the prose. Needs a decision about which caller is wrong, and
  possibly a code change on its own branch.
- **Finding 5** — the admin half of the link handshake is deliberately unspecified. Deciding whether
  the document should now prescribe a shape is a contract decision, not a documentation fix.
- **Finding 6** — nothing throttles guesses at the link code. Whether the document should *require*
  rate limiting of a third-party site is the same class of decision.
- **Finding 10** — TLS certificate requirements. Whether to promise self-signed support is a
  product decision with a support cost attached.
- **Finding 14** — redirects. "Do not depend on a particular behaviour" is not actionable; deciding
  what to promise instead needs a call on what refbox should guarantee.

**Needs a live refbox against a stub (phase 3):**

- **Finding 7** — `filter=Past` exclusive or additive. Settleable only by watching what refbox
  displays for each value.
- **Finding 8** — whether a tokenless call 2 must be refused in the *released* build. The reviewer's
  sharpest finding: the document's most important warning is protected by a guard that exists only
  in unreleased work. Needs a released build pointed at a stub that answers `200`.
- **Finding 31** — whether a site can be checked reachable before a game. Depends on 8.

---

## Deviations

_Record anything that diverged from this plan here, rather than in standalone commits._
