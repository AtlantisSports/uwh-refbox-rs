# Third-Party Integration Document Hardening — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the nine presentation and accuracy defects that make `docs/third-party-integration.md` and its stub unsafe to hand to an outside developer, without touching the technical content that is already correct.

**Architecture:** Pure documentation change plus two small Python comment edits and one file move. Every task is verified by a command whose output is stated in advance — the documentation analogue of a test. No Rust source is modified. No behaviour changes.

**Tech Stack:** Markdown, Python 3 standard library (`check_citations.py`, `stub_site.py`), `ripgrep` + `diff` for the path-drift check.

**Spec:** `docs/audit-archive/2026-08-14-third-party-integration-adversarial-dump.md` in the primary
worktree (`/home/estraily/projects/uwh-refbox-rs`) — the review dump this plan closes. Section
numbers below (§3.2a etc.) refer to it.

**Branch:** `docs/workspace/get-event-team-attribution` in worktree
`/home/estraily/projects/refbox-third-party-contract`. **Not** `docs/workspace/third-party-data-source`,
which is a superseded pre-rebase snapshot.

## Global Constraints

- **Run every command from the worktree root** `/home/estraily/projects/refbox-third-party-contract`. `check_citations.py` hardcodes a repo-root-relative path and exits 2 otherwise.
- **No Rust source changes.** Not `uwh-common/`, not `refbox/`, not `overlay/`. If a fix appears to need one, stop and report.
- **No overlay behaviour change.** Ruled 2026-08-14: the overlay's divergent schedule shape is documented as incompatible; which caller is wrong stays an open decision for the human.
- **Do not close round-4 findings 5, 6, 8, 10, 14 or 31.** Each needs a human decision or a live refbox. Inventing answers is what produced the inverted `games`-key claim on this branch before.
- **Do not restyle prose.** Bolding density, em-dash usage and rhetorical repetition are explicitly out of scope.
- **Commits require approval.** Three commits total, at the end of Tasks 6, 7 and 8. Show the staged diff and wait for the human before each. Never commit on a detached HEAD (the pre-commit hook rejects it).
- **Citation format:** full path first use (`refbox/src/app/mod.rs:1120`), bare filename or bare `:line` afterwards. `check_citations.py` resolves all three forms.

---

### Task 1: Repair the honesty check so it passes and keeps its claim

The document ships a path-drift check that has failed since 2026-08-11 (§5.1) and currently prints a
six-line diff instead of `IN SYNC`. All six extras are typed-address prose, not endpoints. The repair
is verified: normalise concrete event IDs, then exclude the four forms that are not endpoints.

**Files:**
- Modify: `docs/third-party-integration.md:1803-1823`

**Interfaces:**
- Consumes: nothing.
- Produces: a passing `IN SYNC` check that Task 6's final verification re-runs.

- [ ] **Step 1: Confirm the check currently fails**

Run from the worktree root:
```bash
diff \
  <(rg -o -N '/api/[A-Za-z0-9/{}_-]+' uwh-common/src/uwhportal/mod.rs overlay/src/network.rs \
     | sed 's/^[^:]*://; s/{[^}]*}/{}/g' | sort -u) \
  <(rg -o '/api/[A-Za-z0-9/{}_-]+' docs/third-party-integration.md \
     | sed 's/{[^}]*}/{}/g' | sort -u) \
  && echo "IN SYNC"
```
Expected: six `>` lines, exit 1, no `IN SYNC`.

- [ ] **Step 2: Replace the whole "Keeping this document honest" section**

Replace lines 1803–1823 (from the `## Keeping this document honest` heading to end of file) with:

````markdown
## Keeping this document honest

This document can drift from the code. The check below catches one specific kind of drift —
the set of paths — by comparing every `/api/...` path found in the source files against every
`/api/...` path found in this document. Both sides normalise placeholder names (like
`{eventId}`) to a common `{}` so naming differences don't cause false alarms, and the document
side additionally normalises the example event ID `1234-A` to `{}` so a worked example matches
the endpoint it illustrates. Four forms are excluded because they are not endpoints at all —
they are the typed-address convention from
[Pointing refbox at your site](#pointing-refbox-at-your-site), which no site implements:

```bash
diff \
  <(rg -o -N '/api/[A-Za-z0-9/{}_-]+' uwh-common/src/uwhportal/mod.rs overlay/src/network.rs \
     | sed 's/^[^:]*://; s/{[^}]*}/{}/g' | sort -u) \
  <(rg -o '/api/[A-Za-z0-9/{}_-]+' docs/third-party-integration.md \
     | sed 's/1234-A/{}/g; s/{[^}]*}/{}/g' | sort -u \
     | grep -vxF -e '/api/{}' -e '/api/{}/' -e '/api/events/' -e '/api/events/{}') \
  && echo "IN SYNC"
```

It needs [ripgrep](https://github.com/BurntSushi/ripgrep) on the path, and must be run from the
repository root. It should print `IN SYNC` and exit `0`; anything else is real drift.

This only proves the *paths* still match — it says nothing about whether the request or
response bodies documented here still match what the code sends and expects. The real test
of that is rebuilding a working stub server from this document alone and confirming it
actually stands in for the Portal.
````

Three things changed beyond the command: the prose now describes both normalisations and the
exclusions; the ripgrep dependency and the working directory are stated; and the trailing
`(Task 5)` internal reference is gone (§6.5).

- [ ] **Step 3: Verify the check now passes**

Run the command exactly as it now appears in the document.
Expected: `IN SYNC`, exit 0.

- [ ] **Step 4: Verify no internal task references remain**

Run: `grep -n "Task [0-9]" docs/third-party-integration.md`
Expected: no output.

---

### Task 2: Make the `refBoxId` account true and complete

Two defects in one flow: an internal contradiction about width (§5.5) and a missing regeneration
trigger that causes a real pairing failure in the field (§3.2d). The document tells the reader the
number is stable for the run; pressing APPLY on the SITE row mints a new one.

**Files:**
- Modify: `docs/third-party-integration.md:284-291`, `:313`

**Interfaces:**
- Consumes: nothing.
- Produces: the `refbox/src/app/mod.rs:1120` citation, first use of that path in the document — later bare `mod.rs:` citations already resolve to `refbox/src/app/mod.rs` in this region.

- [ ] **Step 1: Fix the regeneration rule (line 286)**

Find:
```
   not validate it as a six-digit string. **It is also regenerated every time refbox restarts**, so
```
Replace with:
```
   not validate it as a six-digit string. **It is also regenerated every time refbox restarts — and,
   on the in-app route, every time the operator presses APPLY on the SITE row**, because the number
   lives on the portal client and applying an address builds a fresh one
   (`refbox/src/app/mod.rs:1120`). If an admin has already registered a pending link and the operator
   then re-applies the address, the number the admin was given is stale and call 1 answers
   `NoPendingLink`. So
```

- [ ] **Step 2: Remove the width contradiction (line 291)**

Find:
```
4. refbox posts `{"refBoxId": "<six digits>", "code": "<code>"}` to call 1.
```
Replace with:
```
4. refbox posts `{"refBoxId": "<the number from step 1>", "code": "<code>"}` to call 1.
```

- [ ] **Step 3: Correct the worked example's aside (line 313)**

Find:
```
refBoxId 482913  →  code 731904   (six digits, no leading zero, expires when you decide)
```
Replace with:
```
refBoxId 482913  →  code 731904   (at most six digits, no leading zero, expires when you decide)
```

- [ ] **Step 4: Verify the contradiction is gone**

Run: `grep -n "six digit" docs/third-party-integration.md`
Expected: exactly three hits — line ~248 (`**At most six digits.**`), and the two in step 1 of the
login flow that say the `refBoxId` *may be shorter than* six digits and must not be validated as a
six-digit string. No hit asserting the posted value *is* six digits.

- [ ] **Step 5: Verify the new citation resolves**

Run: `python3 docs/third-party-stub/check_citations.py | grep -A2 "app/mod.rs:1120"`
Expected: shows `build_site_client` being called, i.e. the line the sentence claims.

---

### Task 3: Define what "unreleased" means

The most important obligation in the document is protected by behaviour that exists only on an
unmerged branch (§4.5 finding 8), and the document never says that "unreleased" means "unmerged", not
"feature flag" (§3.4).

**Propagation decision — one site, not four.** The definition goes in the preamble only. The section
heading (`:24`), the call-2 marker (`:424`, `:480`) and the rules-section marker (`:1027`) all already
say "unreleased" and will now resolve against a stated definition. Adding it at each site would
compound the repetition already flagged in §6.2.

**Files:**
- Modify: `docs/third-party-integration.md:3-8`

- [ ] **Step 1: Add the definition to the preamble**

Find:
```
call 2. Those parts are marked "unreleased" where they appear, and describe a branch that has not
shipped, so treat them as the intended shape rather than something you can test against a download
today.
```
Replace with:
```
call 2. Those parts are marked "unreleased" where they appear, and describe a branch that has not
shipped, so treat them as the intended shape rather than something you can test against a download
today. **"Unreleased" here means exactly one thing: the behaviour exists only on an unmerged
development branch.** There is no build flag, cargo feature, or setting that switches it on — a
downloaded release does not contain it, and there is nothing to enable.
```

- [ ] **Step 2: Verify every "unreleased" marker still resolves to the definition**

Run: `grep -n -i "unreleased" docs/third-party-integration.md`
Expected: the preamble definition plus the existing markers at the route chooser, the section
heading, call 1's token note, call 2's two mentions, and the rules section. No marker left without a
definition earlier in the document.

---

### Task 4: Qualify the four ambiguous `call N` references

80 `call N` references exist across three numbering schemes. Audited: one is genuinely ambiguous,
three are correct but carry no scheme qualifier (§5.4). Adding the call's name resolves all four —
this is not a renumbering.

**Files:**
- Modify: `docs/third-party-integration.md:1619`, `:1681`, `:1698`, `:1789`

- [ ] **Step 1: Line 1619 — coin-flip menu**

Find: `and a winning team from the menu populated by call 5.`
Replace: `and a winning team from the menu populated by call 5 above (get coin flips).`

- [ ] **Step 2: Line 1681 — push team map, when called**

Find:
```
**When schedule-processor calls it:** Immediately after call 7 succeeds, in the same "Upload
```
Replace:
```
**When schedule-processor calls it:** Immediately after call 7 above (push schedule) succeeds, in the same "Upload
```

- [ ] **Step 3: Line 1698 — push team map, on failure**

Find: `**On failure:** Same as call 7: logged, saved login token cleared, back to the main menu.`
Replace: `**On failure:** Same as call 7 above (push schedule): logged, saved login token cleared, back to the main menu.`

- [ ] **Step 4: Line 1789 — the genuinely ambiguous one**

A bullet titled **Team roster** currently says "same as call 9 above". The referent is the other
nine's call 9 (overlay attachments), but the refbox nine's call 9 *is* team roster, so both readings
are available.

Find: `- **Team roster** (`:96`): same as call 9 above — a connection failure, or a body that isn't`
Replace: `- **Team roster** (`:96`): same as call 9 above (overlay attachments) — a connection failure, or a body that isn't`

- [ ] **Step 5: Verify no unqualified back-reference remains in "the other nine"**

Run:
```bash
awk 'NR>=1438' docs/third-party-integration.md \
  | grep -n "call [0-9]" | grep -v "above\|below\|refbox nine\|other nine\|these nine"
```
Expected: no output.

---

### Task 5: Correct three citations that resolve without supporting their claim

`check_citations.py` proves a cited line exists, not that it means what the sentence says (§3.2).
These three were found by reading the cited lines.

**Files:**
- Modify: `docs/third-party-integration.md:1013`, `:1117-1118`, `:1369`

- [ ] **Step 1: Line 1013 — the stats body/header citation spans two lines**

The sentence claims two actions (serialises its own body; sets the header explicitly). Line 335 is
`.body(...)`; line 336 is `.header("Content-Type", ...)`.

Find: `header explicitly (`mod.rs:335`). What arrives on the wire is the same either way.`
Replace: `header explicitly (`mod.rs:335-336`). What arrives on the wire is the same either way.`

- [ ] **Step 2: Lines 1117-1118 — there are two error messages, not one**

`schedule.rs:706` says `'events/'`; `schedule.rs:754` says `'teams/'`. The section covers both ID
types, so "always … 'events/'" is false for the likelier case.

Find:
```
one malformed ID in an event list costs you the whole list. And the message is always `Invalid
format for full_id. It should start with 'events/'`, even when the prefix was perfectly fine and
the length was the real problem. If you see that error against an ID that plainly starts with
`events/`, count the characters after the slash.
```
Replace:
```
one malformed ID in an event list costs you the whole list. And the message names only the prefix,
never the length: `Invalid format for full_id. It should start with 'events/'` for an event ID
(`schedule.rs:706`) and the same sentence ending `'teams/'` for a team ID (`schedule.rs:754`). You
get that message even when the prefix was perfectly fine and the length was the real problem, so if
you see it against an ID that plainly starts with the right prefix, count the characters after the
slash.
```

- [ ] **Step 3: Line 1369 — drop the pass-through citation**

`refbox/src/tournament_manager/mod.rs:2182-2188` is `game_clock_time`, which just delegates to
`self.clock_state.clock_time(now)`. It supports nothing about count-up behaviour; `:1855-1868` alone
does.

Find: `not remaining (`refbox/src/tournament_manager/mod.rs:1855-1868`, `:2182-2188`). |`
Replace: `not remaining (`refbox/src/tournament_manager/mod.rs:1855-1868`). |`

- [ ] **Step 4: Verify all citations still resolve**

Run: `python3 docs/third-party-stub/check_citations.py | tail -2`
Expected: `N citations checked, 0 unresolvable`, exit 0.

---

### Task 6: Replace the impossible instruction, then commit the document work

The document tells a site to satisfy two mutually exclusive shapes (§4.5 finding 3). Per the
2026-08-14 ruling: state the incompatibility, change no code, leave the which-caller-is-wrong
question to the human.

**Files:**
- Modify: `docs/third-party-integration.md:1782-1783`

- [ ] **Step 1: Replace the closing sentence of the public-schedule bullet**

Find:
```
  document. A site serving both the overlay and schedule-processor/refbox from the same schedule
  response needs to satisfy both shapes at once, or one of the callers won't find what it needs.
```
Replace:
```
  document. **These two shapes are mutually exclusive — `games` cannot be both an object keyed by
  game number and a plain array — so one schedule response cannot satisfy both the overlay and
  schedule-processor/refbox.** This is a divergence inside refbox's own code, not a shape a site is
  expected to reconcile: treat the overlay's schedule reading as a separate, incompatible consumer,
  and serve it from its own response if you need it at all. A site that only stands in for the
  refbox during a game never has to resolve this.
```

- [ ] **Step 2: Verify the impossible instruction is gone**

Run: `grep -n "both shapes at once" docs/third-party-integration.md`
Expected: no output.

- [ ] **Step 3: Re-run both self-checks over the finished document**

Run: `python3 docs/third-party-stub/check_citations.py | tail -2`
Expected: `0 unresolvable`, exit 0.

Run the honesty check from Task 1 Step 3.
Expected: `IN SYNC`, exit 0.

- [ ] **Step 4: Show the staged diff and WAIT for approval**

```bash
git add docs/third-party-integration.md
git diff --cached --stat
git diff --cached
```
Do not proceed until the human approves. Summarise in plain English: what changed, why, and that the
document's own two checks now both pass.

- [ ] **Step 5: Commit**

```bash
git commit -m "docs(workspace): harden the third-party contract for external release"
```
Body must record: the honesty check had been failing since 2026-08-11 and now passes with the
typed-address forms excluded; the `refBoxId` also regenerates on APPLY, which no earlier revision
said; "unreleased" means unmerged with no flag; four `call N` references gained a name; three
citations resolved without supporting their claim; and the `games`-both-shapes instruction was
impossible and is now stated as an incompatibility.

---

### Task 7: Move the round-4 ledger out of the deliverable directory

Runs **before** Task 8 so the README rewrite can cite the ledger's final path.

`ROUND-4-FINDINGS.md` sits inside `docs/third-party-stub/`, where a reader browsing the contract
finds an internal review ledger listing seven unclosed items (§1). It is the only tracked ledger —
rounds 2 and 3 live under the gitignored `.superpowers/` — so it moves rather than being deleted.

**Files:**
- Move: `docs/third-party-stub/ROUND-4-FINDINGS.md` → `docs/superpowers/plans/2026-08-13-round-4-findings.md`

- [ ] **Step 1: Find every reference to the file before moving it**

Run: `grep -rn "ROUND-4-FINDINGS" --exclude-dir=.git --exclude-dir=target .`
Record each hit; each needs updating in Step 3.

- [ ] **Step 2: Move it with git so history follows**

```bash
git mv docs/third-party-stub/ROUND-4-FINDINGS.md \
       docs/superpowers/plans/2026-08-13-round-4-findings.md
```

- [ ] **Step 3: Update every reference found in Step 1**

If Step 1 found none outside the file itself, this step is a no-op — say so rather than inventing a
reference. Task 8 adds the only intended pointer, and it names the new path directly.

- [ ] **Step 4: Verify nothing points at the old path**

Run: `grep -rn "third-party-stub/ROUND-4-FINDINGS" --exclude-dir=.git --exclude-dir=target .`
Expected: no output.

Run: `git ls-files docs/third-party-stub`
Expected: `README.md`, `check_citations.py`, `stub_site.py` — three files, no ledger.

- [ ] **Step 5: Show the staged diff, WAIT for approval, then commit**

```bash
git add -A docs/
git diff --cached --stat
git commit -m "docs(workspace): move the round-4 ledger out of the stub directory"
```

---

### Task 8: Rewrite the stub README and fix the stub's two stale comments

Runs **after** Task 7 so the README can point at the ledger's final path without a forward reference.

`README.md` was last touched 2026-08-10; `stub_site.py` changed on 08-11 and 08-13, so the README
describes a stub that no longer exists (§5.7), and both files cite a "gap 13" the document does not
have (§5.6).

**Files:**
- Modify: `docs/third-party-stub/README.md` (rewrite in place)
- Modify: `docs/third-party-stub/stub_site.py:40`, `:164`

- [ ] **Step 1: Fix the stub's roster comment (line 40)**

Find: `# Hardcoded fake event: one court, two games, two teams of six players each.`
Replace: `# Hardcoded fake event: one court, two games, two teams of six playing members --`
followed by a second comment line: `# plus one coach-only member on the second team, to exercise the role filter.`

- [ ] **Step 2: Remove the dangling "gap 13" reference (line 164)**

Find: `    integration document as gap 13.`
Replace: `    stated in the integration document under call 2, "What your site must do".`

- [ ] **Step 3: Rewrite `README.md`**

Keep: the one-reason framing, the blind-build rationale (it is the most valuable part of the file),
the run instructions, the config-backup warning, and the "expect it to rot" scoping. Fix all six
defects from §5.7:

- Replace *"a snapshot of what the document described on 2026-08-10"* with the real state: last
  rebuilt against the document on 2026-08-13, serving nine calls.
- Replace *"Seventeen gaps came out of it"* with a statement that does not go stale: say that every
  round's findings were fixed in the document rather than worked around here, and point at
  `docs/superpowers/plans/2026-08-13-round-4-findings.md` — the path Task 7 moved it to — for the
  latest round's ledger, rather than quoting a count that goes stale.
- Replace *"two teams of six players"* with the truth: two teams of six playing members, one of whom
  also coaches, plus a coach-only member on the second team who must **not** reach the grid.
- Add call 9 (`GET /api/admin/get-event-team`) to "What it serves" — the stub has served it since
  2026-08-13 and the README never mentions it.
- Delete the *"That is gap 13 in the document"* sentence; refer to call 2's obligation instead.
- Rewrite the self-contradicting opener *"**It does not accept any bearer token, and that is the
  point.**"* — it is followed immediately by a sentence saying it accepts exactly one. Use:
  *"**It accepts exactly one bearer token — the access key it issued itself — and refuses everything
  else, including a request with no `Authorization` header at all.**"*
- Keep the explanation of why refusing matters (an earlier version accepted everything, refbox
  decided it was already paired and never offered to link), but describe it without the gap number.
- Add one sentence the README currently lacks: the stub deliberately does not implement call 1's
  pairing negotiation, and `stub_site.py`'s own banner says so — readers must not copy that handler.

- [ ] **Step 4: Verify the stub still runs and the citations to it still resolve**

Run: `python3 -c "import ast,pathlib; ast.parse(pathlib.Path('docs/third-party-stub/stub_site.py').read_text())" && echo "parses"`
Expected: `parses`

Run: `python3 docs/third-party-stub/check_citations.py | grep -A2 "stub_site.py"`
Expected: `stub_site.py:365-366` still shows `class Handler` / `protocol_version = "HTTP/1.1"` and
`:436` still shows `ThreadingHTTPServer` — the document cites these line numbers, so if the comment
edits in Steps 1–2 shifted them, update the document's citations and re-run
`check_citations.py`.

- [ ] **Step 5: Verify no "gap N" reference survives anywhere**

Run: `grep -rn -i "gap 1\?[0-9]" docs/third-party-stub/ docs/third-party-integration.md`
Expected: no output.

- [ ] **Step 6: Show the staged diff, WAIT for approval, then commit**

```bash
git add docs/third-party-stub/README.md docs/third-party-stub/stub_site.py
git diff --cached
git commit -m "docs(workspace): rewrite the stub README to match the stub it documents"
```

---

## Final verification

- [ ] `python3 docs/third-party-stub/check_citations.py` → `0 unresolvable`, exit 0
- [ ] The honesty check as written in the document → `IN SYNC`, exit 0
- [ ] `grep -n "Task [0-9]" docs/third-party-integration.md` → no output
- [ ] `grep -rn -i "gap 1\?[0-9]" docs/third-party-stub/ docs/third-party-integration.md` → no output
- [ ] `grep -n "both shapes at once" docs/third-party-integration.md` → no output
- [ ] `git status` → clean, three commits ahead of the branch's previous tip
- [ ] Run `superpowers:requesting-code-review` once, over the three commits together (lean process per
      `.claude/rules/plan-execution.md`: one review per feature, not per task)

## Out of scope — do not do these

- Round-4 findings 5, 6, 8, 10, 14, 31 (need a human decision or a live refbox)
- Any overlay code change (finding 3 stays a documentation statement)
- Prose restyling: bolding density, em-dashes, rhetorical repetition
- Rewriting the superseded `docs/workspace/third-party-data-source` branch
- Pushing, or opening a PR

## Deviations

_(Record any divergence from this plan here rather than in a separate commit, per
`.claude/rules/plan-execution.md`.)_

**Task 2 — the plan's Interfaces block was wrong, and adding a citation broke another one.**
The plan asserted that "later bare `mod.rs:` citations already resolve to `refbox/src/app/mod.rs`
in this region". They did not: in that region the shorthand anchored to
`uwh-common/src/uwhportal/mod.rs`. Introducing `refbox/src/app/mod.rs:1120` at doc:289 re-anchored
the next shorthand citation, so `` `mod.rs:236-248` `` at doc:303 — the `NoPendingLink` /
`InvalidCode` match — silently began resolving to `refbox/src/app/mod.rs:236-248`, an unrelated doc
comment. `check_citations.py` still reported `0 unresolvable`, because it was resolvable, just wrong.

Fixed by writing that citation out in full (`uwh-common/src/uwhportal/mod.rs:236-248`), which also
restores the anchor for every shorthand after it. Verified by diffing the checker's resolved paths
against a pre-edit baseline: the only differences are the intended new citation and that one
shorthand becoming explicit at the same path it had before.

**Carry this into any future edit:** adding a full-path citation for a *second* file whose basename
is already used in shorthand nearby will hijack the next shorthand. Diff the checker's resolved
paths against a baseline rather than trusting the `0 unresolvable` count — that count cannot see
this class of error, which is the same "resolves but does not support" failure the branch already
recorded once.
