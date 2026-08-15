# Third-Party Contract: Ruling Implementation and Finding 3 Investigation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the last open findings on the third-party contract — implement the six document changes the human's five rulings imply, record those rulings in a tracked file, and settle finding 3 with evidence from the live Portal instead of a source comment of unknown age.

**Architecture:** Documentation only. One task gathers empirical evidence with unauthenticated `GET`s against the dev and production Portals and corrects the document from what comes back; four tasks apply rulings to specific passages; one records the rulings. No code changes anywhere.

**Tech Stack:** Markdown; `curl` + Python 3 (standard library) for the Portal probe; the two existing self-checks (`check_citations.py`, the in-document path-drift snippet) plus a per-occurrence citation trace.

**Spec:** The five rulings, given 2026-08-15 and reproduced verbatim in Task 5. Their evidence base is
`docs/audit-archive/2026-08-14-third-party-integration-adversarial-dump.md` in the primary worktree
and `docs/superpowers/plans/2026-08-13-round-4-findings.md` here.

**Branch:** `docs/workspace/get-event-team-attribution` in worktree
`/home/estraily/projects/refbox-third-party-contract`.

## Global Constraints

- **Run everything from the worktree root.** `check_citations.py` hardcodes a repo-root-relative path.
- **No code changes.** Not `refbox/`, not `overlay/`, not `uwh-common/`, not `schedule-processor/`. Finding 3's investigation produces evidence and a recommendation; whether the overlay changes is a separate decision on its own branch.
- **The Portal probe is read-only and unauthenticated.** Only `GET` requests, and only to paths the document itself marks `Auth: none` — `/api/events` and `/api/events/{eventId}/schedule`. **Never send credentials, never POST, never touch `/schedule/privileged`.** Production is in scope for reading precisely because the question is what the *real* Portal serves.
- **`limit` must be between 5 and 200** on `/api/events` — a `limit=1` probe returns `400` with a validation body. refbox always sends `100`.
- **Do not attach a `file:line` citation to any claim about the released v0.4.9 binary.** `check_citations.py` resolves citations against the *working tree*, where the tokenless-verify guard is present — the opposite of what the claim says about the release. Name the release by version in prose instead. This is the one place in the document where a resolvable citation would actively mislead.
- **Re-run the per-occurrence citation trace, not just the `0 unresolvable` count,** after any task that adds a citation. Adding a full-path citation re-anchors the next shorthand one; that has happened three times on this branch, and the checker reports success every time. The trace script lives in the session scratchpad as `trace_citations.py`, not in the repo — if it is missing, recreate it: it applies `check_citations.py`'s own resolution rules but prints **one line per citation occurrence** (resolved path, span, and the cited line's text) with **no deduplication**, taking an optional git ref so two revisions can be diffed. The deduplication is exactly what hides a re-anchor whose `(path, start, end)` key appears elsewhere.
- **Prose wraps at ~100 characters.** Match the file; do not reflow lines you did not otherwise touch.
- **Commits require approval.** Three commits, at the end of Tasks 1, 4 and 5. Show the staged diff and wait.

---

### Task 1: Settle finding 3 with live evidence, then correct the document

The document says the public schedule endpoint returns "the same shape" as the privileged one. The
overlay reads that same path as a **plain array** with `dark.assignment.teamId`. A source comment
(`uwh-common/src/uwhportal/mod.rs:643-646`) says the Portal returns an array "for some events". All
three cannot be right. Find out which is.

**Files:**
- Create: `/tmp/claude-.../scratchpad/probe_schedule_shapes.py` (scratchpad, not committed)
- Modify: `docs/third-party-integration.md` — the "2. Public event schedule" entry (~`:1503-1523`)

**Interfaces:**
- Produces: an evidence table (event count by shape) that Task 1's document edit cites in prose, and a recommendation recorded in Task 5's rulings file.

- [ ] **Step 1: Write the probe**

Standard library only. For each of dev and prod: fetch both event-list filters, then fetch each
event's public schedule and classify it.

```python
#!/usr/bin/env python3
"""Classify what the Portal serves for `games` on the PUBLIC schedule endpoint.

Read-only. Unauthenticated GETs only, to paths the contract marks Auth: none.
"""
import json, sys, urllib.request
from collections import Counter

HOSTS = {"dev": "https://api.dev.uwhportal.com", "prod": "https://api.uwhportal.com"}


def get(url):
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.loads(r.read().decode())


def classify(sched):
    games = sched.get("games")
    if isinstance(games, list):
        shape, sample = "array", (games[0] if games else None)
    elif isinstance(games, dict):
        shape, sample = "object", (next(iter(games.values())) if games else None)
    else:
        return f"games-missing({type(games).__name__})", None
    team = "n/a"
    if isinstance(sample, dict):
        dark = sample.get("dark")
        if isinstance(dark, dict):
            team = "dark.assignment.teamId" if "assignment" in dark else \
                   ("dark.teamId" if "teamId" in dark else f"dark:{sorted(dark)[:3]}")
    top = [k for k in ("court", "startsOn") if k in sched]
    return shape, (team, tuple(top))


for env, base in HOSTS.items():
    ids = []
    for filt in ("InProgressOrUpcoming", "Past"):
        try:
            lst = get(f"{base}/api/events?limit=100&filter={filt}&isSchedulePublished=true")
        except Exception as e:
            print(f"{env}/{filt}: LIST FAILED {e}"); continue
        ids += [i["id"].split("/", 1)[-1] for i in lst.get("items", [])]
    ids = sorted(set(ids))
    print(f"\n=== {env}: {len(ids)} events with a published schedule ===")
    tally = Counter()
    for eid in ids:
        try:
            sched = get(f"{base}/api/events/{eid}/schedule")
        except Exception as e:
            tally[f"fetch-failed:{getattr(e, 'code', e)}"] += 1
            continue
        shape, detail = classify(sched)
        tally[(shape, detail)] += 1
        print(f"  {eid:10} {shape:8} {detail}")
    print(f"  --- tally: {dict(tally)}")
```

- [ ] **Step 2: Run it and record the result**

Run: `python3 <scratchpad>/probe_schedule_shapes.py 2>&1 | tee <scratchpad>/shapes.txt`

Expected: a per-event classification and a tally per environment. **Whatever comes back is the
answer** — do not massage it to match the source comment. Three outcomes are possible and each
implies different prose in Step 3:

- **All object, both environments** → the source comment is stale, the overlay is reading a shape the Portal no longer serves, and the overlay is the defect.
- **Mixed** → the comment is accurate, each caller copes differently, and neither is simply wrong.
- **All array** → the *document* is wrong to say public and privileged return the same shape, and schedule-processor's object-shaped parse is what fails.

If every fetch fails (no network, endpoint moved), **stop and report** — do not write conclusions
from a failed probe. That is the same error as trusting a citation because it resolves.

- [ ] **Step 3: Correct the public-schedule entry from the evidence**

Rewrite the **Successful response** paragraph of "#### 2. Public event schedule". Current text:

```
**Successful response:** `200` with the same shape documented under
[the schedule payload](#the-schedule-payload) for the privileged call — this endpoint and the
privileged one return the same shape, just gated differently.
```

Required content for the replacement, whatever the evidence says:
- State the shape this endpoint **actually** returns, as observed, with the date and how many events were checked in each environment. Use the document's existing evidence idiom — "observed live on DATE" — which already appears at `:471` and `:1391`.
- If the shape differs from the privileged call's, say so plainly and drop the "same shape" claim. If it matches, say the overlay's array reading is not what the endpoint serves today.
- Keep the existing "When schedule-processor calls it" paragraph's account of the array fallback, but reconcile it with the evidence rather than leaving both claims standing.
- Do **not** state which caller should change. That decision is the human's and is out of scope.

Also update the overlay's public-schedule bullet (`:1794-1802`, the one this branch rewrote) if the
evidence contradicts what it now says.

- [ ] **Step 4: Verify**

Run: `python3 docs/third-party-stub/check_citations.py | tail -1` → `0 unresolvable`, exit 0.
Run the in-document honesty check → `IN SYNC`.
Run the per-occurrence trace and diff against `HEAD` → only intended additions.

- [ ] **Step 5: Show the staged diff, WAIT for approval, then commit**

```bash
git add docs/third-party-integration.md
git diff --cached
git commit -m "docs(workspace): settle the public schedule's shape against the live portal"
```

Body must record: what was probed (both environments, both filters, N events each), the tally, and
what it means for the overlay-vs-schedule-processor question — explicitly leaving that decision open.

---

### Task 2: State the real transport behaviour — redirects and TLS

Findings 14 and 10. Both are HTTP-client facts the document currently waves at. Nothing in the
workspace configures either: the client is built with only `https_only` and `timeout`
(`uwh-common/src/uwhportal/mod.rs:172-175`), so both are reqwest 0.12 defaults.

**Files:**
- Modify: `docs/third-party-integration.md:955-961` (the "HTTP-level facts, in one place" paragraph)

- [ ] **Step 1: Replace the redirect sentence**

Find:
```
as a transport failure, so a site that takes longer to answer is indistinguishable from a site that
is down. Redirects, keep-alive and connection reuse are whatever the underlying HTTP client does by
default — nothing is configured, so do not build a site that depends on a particular behaviour
there. Requests are not serialised: selecting an event triggers a burst (teams and schedule
```

Replace with text carrying this required content, wrapped at ~100 chars:
- The 10-second timeout sentence is unchanged.
- **Redirects are followed** — up to ten, reqwest's default, because nothing configures a policy. So an `nginx` canonicalisation or an http→https redirect works, and a site does not have to serve everything from the exact URL it was given.
- **A redirect that downgrades to plain `http` is refused** whenever TLS is required, which is the default — the same `https_only` setting that refuses a plain-http base URL also refuses a downgrade mid-redirect.
- Keep-alive and connection reuse remain "whatever the client does by default".
- **Label the whole thing as observed behaviour of a dependency default, not a promise** — a future dependency bump could change it, and the document has no stability promise anyway (see its preamble). This is the honest version of the sentence it replaces; "do not depend on a particular behaviour" was weaker than the truth and left an implementer unable to act.
- Keep the existing "Requests are not serialised…" sentence that follows.

- [ ] **Step 2: Add the certificate paragraph**

Insert a new paragraph immediately after the HTTP-level-facts paragraph. Required content:
- **Certificates are validated normally**, against the trust store of the machine running refbox — nothing overrides reqwest's default, so there is no "accept any certificate" mode and no way for a site to ask for one.
- **A self-signed certificate is therefore rejected**, and per the rule already documented for plain `http`, the failure is indistinguishable from the site being unreachable — nothing on screen or in the log names the certificate.
- **The supported route for a site with no public DNS name** — the ordinary pool-LAN case — is to generate your own certificate authority, install it in the trust store of the machine running refbox, and serve `https` normally. This keeps the access key encrypted, which is the reason TLS is required in the first place.
- **On a Raspberry Pi this is a real setup step, not a one-liner:** the deployment image runs a read-only overlay filesystem, so the CA has to be baked into the image or the overlay remounted writable to install it.
- The alternative remains plain `http` with `--allow-http` on a network you trust, already documented under [The environment override](#the-environment-override-built-in-portal-only).

Cite `uwh-common/src/uwhportal/mod.rs:172-175` once for "nothing overrides the defaults" — it is
already cited elsewhere in this section, so confirm the shorthand anchoring still resolves after
the edit.

- [ ] **Step 3: Verify**

Run: `python3 docs/third-party-stub/check_citations.py | tail -1` → `0 unresolvable`, exit 0.
Run the in-document honesty check → `IN SYNC`.
Run the per-occurrence trace and diff against the previous commit — this task adds a citation to a
file already cited nearby, so a re-anchor is exactly the risk here; the `0 unresolvable` count will
not show it.
Confirm no added prose line exceeds 100 characters.

---

### Task 3: Say what the released build actually does, and how to prove a site is reachable

Findings 8 and 31.

**Files:**
- Modify: `docs/third-party-integration.md` — call 2's "Unreleased:" passage (~`:491-496`) and its "On failure" region

- [ ] **Step 1: Add the released-build consequence**

Immediately after the paragraph beginning "**Unreleased: refbox has closed the worst version of
that, and only the worst version.**", add a paragraph with this required content:

- **In the build you can download today, this narrowing does not exist.** refbox v0.4.9 — the current release — sends this call as soon as an event is selected, whether or not it holds a token. The guard described above lives only in the unreleased custom-source work.
- So a site that answers `200` to an unauthenticated verify **reproduces the bypass in full** against a released refbox: green token row, privileged schedule loaded, court filled in, link flow never offered, nothing reported anywhere.
- **This is the single most consequential rule in this document**, and the protection a reader might infer from the paragraph above is not in their hands yet.
- **No `file:line` citation on this paragraph.** Per the Global Constraints: the checker resolves against the working tree, where the guard *is* present, so a citation here would resolve to code that contradicts the sentence. Name the version instead.

- [ ] **Step 2: Add the reachability answer**

Add a short paragraph (finding 31) stating that a site operator does not need a token, or anyone to
have linked, to prove the site is reachable: turning Portal mode on fires call 3 (event list) and
then call 4 (teams) for every event returned, both unauthenticated, so requests arrive in the site's
log before any pairing happens. What *does* depend on call 2 is the operator-visible indicator —
which is why a site can be up, answering, and still show red until a token exists.

- [ ] **Step 3: Verify**

Run: `grep -n "v0.4.9" docs/third-party-integration.md` — expect the preamble's existing mention plus
the new one, and **no `file:line` citation on the same line as either**.
Run the citation checker and the honesty check; both must pass.

---

### Task 4: Require throttling, and show one shape for the admin half

Findings 6 (ruled: **normative requirement**) and 5 (ruled: **non-normative worked example**).
These are one region of call 1 and one reviewer gate.

**Files:**
- Modify: `docs/third-party-integration.md` — the "Concretely, what implementing it means:" list (~`:378-396`) and the passage following it

- [ ] **Step 1: Add the throttling obligation to the existing list**

The list's bullets are already normative and imperative ("Record…", "Issue…", "Reject…", "Expire…",
"Keep…"). Insert a new bullet after the "Expire codes, and make a code single-use…" bullet:

```
- **Throttle failed code attempts, and stop accepting them after a small number.** At most 900,000
  codes exist, the `refBoxId` is on the refbox screen for anyone in the room to read, and a wrong
  guess costs an attacker nothing but one request. Without a limit, a pairing your admin has just
  issued can be brute-forced while the operator is still typing it. How you throttle is yours — a
  per-`refBoxId` attempt count, a growing delay, or a lockout that forces a fresh registration —
  but a site with no limit at all is not safe to run. refbox cannot tell whether you did this;
  nothing in these nine calls would look different either way.
```

- [ ] **Step 2: Add the non-normative worked example of the admin half**

The bullet stating "**The admin half of this handshake is yours entirely**" **stays exactly as it
is** — that is the ruling. Add a short worked example *after* the list, framed so it cannot be read
as a requirement. Required content:

- An explicit label that this is one shape that works, not a specification, and that nothing here is checked by refbox.
- The four things the sealed-room tester had to invent, presented as a sequence: an admin-only screen that accepts a `refBoxId`; recording a pending link for it; issuing a code with an expiry; consuming the pending link when the code is redeemed.
- A sentence tying it back to the obligations above — the example satisfies every bullet in the list, including the new throttling one, and is the shortest thing that does.
- **No shapes, no endpoint paths, no JSON.** Inventing an admin API is exactly what the ruling declined to do; this is a description of a flow, not a contract.

- [ ] **Step 3: Verify**

Run: `grep -c "yours entirely" docs/third-party-integration.md` → `1` (the bullet survived unchanged).
Run the citation checker and honesty check; both pass. Confirm no line over 100 chars was added.

- [ ] **Step 4: Show the staged diff, WAIT for approval, then commit**

```bash
git add docs/third-party-integration.md
git diff --cached
git commit -m "docs(workspace): close the last six contract findings"
```

Body must name each of the six findings closed by Tasks 2–4 (5, 6, 8, 10, 14, 31), its ruling, and —
for 14 and the factual half of 10 — that they were resolved from source rather than by decision.

---

### Task 5: Record the rulings, then verify the whole document

The SDD ledger is gitignored; `b3588756` exists because rulings written only there would be lost to
`git clean -fdx`.

**Files:**
- Create: `docs/superpowers/plans/2026-08-15-contract-rulings.md`
- Modify: `docs/superpowers/plans/2026-08-13-round-4-findings.md` — status lines for findings 5, 6, 8, 10, 14, 31

- [ ] **Step 1: Write the rulings file**

Record all five decisions of 2026-08-15, each with the option chosen, the reasoning given in the
question, and what was implemented. Verbatim rulings:

1. **Finding 5** — non-normative worked example; "this half is yours entirely" stays the rule.
2. **Finding 6** — **required normatively**, as a new bullet in the obligations list. *Recommendation had been a labelled recommendation; the human chose the stronger form.*
3. **Finding 10** — document the OS trust-store route; no code change.
4. **Finding 3** — establish what the Portal actually serves before deciding; no code change yet.
5. **Findings 8/31** — document the consequence now, citing the released version.

Also record the two findings that came off the list **without** a ruling, because they were
answerable from source — finding 14 (redirects) and the factual half of finding 10 (certificates) —
and that finding 8's factual half was settled the same way, by comparing the `v0.4.9` tag against
`HEAD`. Note the pattern explicitly: **three findings escalated to the human on this branch turned
out to be answerable from the code**, `filter=Past` being the first. That is the lesson worth
carrying, not the individual answers.

- [ ] **Step 2: Update the round-4 ledger's status lines**

For findings 5, 6, 8, 10, 14 and 31, append a dated line recording the resolution and pointing at
the rulings file — matching the style of finding 3's existing "**Updated 2026-08-14**" note. Do not
rewrite the original finding text; ruling 4 of the phase-1 plan keeps the reviewer's wording.

- [ ] **Step 3: Full verification**

```bash
python3 docs/third-party-stub/check_citations.py | tail -1     # 0 unresolvable, exit 0
grep -c "OPEN" docs/superpowers/plans/2026-08-13-round-4-findings.md
```
Plus the in-document honesty check → `IN SYNC`, and a per-occurrence citation trace diffed against
the commit before Task 1 — every difference must be an intended addition.

- [ ] **Step 4: Show the staged diff, WAIT for approval, then commit**

```bash
git add docs/superpowers/plans/
git diff --cached
git commit -m "docs(workspace): record the 2026-08-15 contract rulings"
```

---

## Final verification

- [ ] `python3 docs/third-party-stub/check_citations.py` → `0 unresolvable`, exit 0
- [ ] The honesty check as written in the document → `IN SYNC`, exit 0
- [ ] Per-occurrence citation trace vs. the pre-Task-1 commit → only intended additions
- [ ] No claim about the released binary carries a `file:line` citation
- [ ] `grep -c "yours entirely"` → 1
- [ ] No added prose line over 100 characters
- [ ] `git status` → clean, three commits ahead
- [ ] Run `superpowers:requesting-code-review` once over all three commits (lean process: one review per feature)

## Out of scope — do not do these

- Any code change, in any crate — including the overlay, whatever Task 1 finds
- Converting the ~60 shorthand citations to full paths (the recommended next pass)
- Prose restyling: bolding density, em-dashes, rhetorical repetition
- The "reference stand-in" vs "NOT A REFERENCE IMPLEMENTATION" contradiction (`:985` vs `stub_site.py:15`)
- Adding `__pycache__` to `.gitignore` (shared config; separate branch)
- Pushing, or opening a PR

## Deviations

_(Record any divergence here rather than in a separate commit, per `.claude/rules/plan-execution.md`.)_
