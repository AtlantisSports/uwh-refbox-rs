# Third-Party Integration Contract Document — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce `docs/third-party-integration.md` — a document complete and accurate
enough that a developer with no access to this repository can build a site the refbox
will run a full game against — and prove it by building exactly that stub.

**Architecture:** Documentation deliverable plus a throwaway verification stub. No Rust
source changes. The document is written from the code; the stub is then written **from
the document alone**, and every failure of the stub against a real refbox is treated as
a documentation bug, never a stub bug. That inversion is what makes this testable rather
than merely proofread.

**Tech Stack:** Markdown; Python 3 standard library only (`http.server`, `json`) for the
stub — no pip installs, not part of the cargo workspace, never run in CI.

## Global Constraints

- Deliverable path: `docs/third-party-integration.md`. Stub: `docs/third-party-stub/`.
- **No Rust source files are modified by this plan.** If a task appears to require one,
  stop and raise it — that is a spec deviation.
- Every documented call must cite its source line, e.g. `uwh-common/src/uwhportal/mod.rs:206`.
- The document opens with an accuracy marker: **"Accurate as of refbox v0.4.9"**
  (`refbox/Cargo.toml:3`). Update the version if it changes during this work.
- The commitment level is **best effort**. The document must say so explicitly and must
  not promise stability.
- Spec of record: `docs/superpowers/specs/2026-08-07-third-party-data-source-design.md`.

## Ground truth established during planning

Do not re-derive these; they are verified and are inputs to the tasks.

**18 operations sit on 16 distinct paths.** Two paths carry two operations each:
coin flips (read and write) and `/schedule` (public read and upload).

**Only eight of the eighteen are refbox's.** This is the document's headline:

| # | Operation | Path | Auth |
|---|---|---|---|
| 1 | Link a refbox | `POST /api/events/{id}/access-keys/ref-box` | none |
| 2 | Verify token | `GET /api/events/{id}/access-keys/verify` | bearer |
| 3 | Event list | `GET /api/events` | none |
| 4 | Event teams | `GET /api/events/{id}/teams` | none |
| 5 | Schedule (privileged) | `GET /api/events/{id}/schedule/privileged` | bearer |
| 6 | Referees | `GET /api/events/{id}/referees` | none |
| 7 | Push scores | `POST /api/events/{id}/schedule/games/{n}/scores` | bearer |
| 8 | Push stats | `POST /api/admin/events/stats` | bearer |

The other ten: `POST /api/authentication`, `GET /api/events/{id}/schedule`,
`GET /api/admin/get-event-team`, `GET /api/events/{id}/participants`,
`GET /api/admin/events/game-referees`, `GET` and `POST /api/events/{slug}/schedule/coin-flips`,
`POST /api/events/{slug}/schedule`, `POST /api/events/{slug}/schedule/map-teams`
(all schedule-processor), and `GET /api/admin/events/{id}/overlay-attachments` (overlay).

**The two ID forms follow one rule, verified across every call site:**
- An event ID **in a URL path** is always the short form — `1889-B` (`partial()`).
- An event or team ID **in a query parameter** is always the long form — `events/1889-B`,
  `teams/10753-A` (`full()`). Only three calls do this: push stats
  (`mod.rs:334`), team roster (`mod.rs:676`), game referees (`mod.rs:779`).

State that rule once in the document. It converts the spec's "not guessable, only
documented" gotcha into a single sentence.

**Running refbox against a local stub** requires the `--allow-http` flag
(`refbox/src/main.rs:154`), because the client otherwise refuses non-HTTPS addresses:

```bash
UWH_PORTAL_URL_OVERRIDE=http://localhost:8099 cargo run -p refbox -- --allow-http
```

## File Structure

| File | Responsibility |
|---|---|
| `docs/third-party-integration.md` | The contract. Overview, the refbox eight, data formats, the other ten, drift-check instructions. |
| `docs/third-party-stub/stub_site.py` | Single-file Python 3 stub answering the refbox eight. Verification only. |
| `docs/third-party-stub/README.md` | How to run the stub and what it proves. |

---

### Task 1: Document skeleton and the endpoint inventory

**Files:**
- Create: `docs/third-party-integration.md`

**Interfaces:**
- Produces: the inventory table and the accuracy marker that Tasks 2 and 4 fill in
  behind, and the `diff` drift check that every later task re-runs.

- [ ] **Step 1: Write the failing check**

Save this as the drift check. It compares the paths the code calls against the paths
the document claims, normalising placeholder names on both sides:

```bash
diff \
  <(rg -o -N '/api/[A-Za-z0-9/{}_-]+' uwh-common/src/uwhportal/mod.rs overlay/src/network.rs \
     | sed 's/^[^:]*://; s/{[^}]*}/{}/g' | sort -u) \
  <(rg -o '/api/[A-Za-z0-9/{}_-]+' docs/third-party-integration.md \
     | sed 's/{[^}]*}/{}/g' | sort -u) \
  && echo "IN SYNC"
```

- [ ] **Step 2: Run it to confirm it fails**

Run the command above.
Expected: it fails — `docs/third-party-integration.md` does not exist yet, so rg
reports no such file and `diff` reports 16 lines missing on the right.

- [ ] **Step 3: Write the skeleton and inventory**

Create `docs/third-party-integration.md` with, in this order:

1. **Title and accuracy marker** — "Accurate as of refbox v0.4.9. This is a best-effort
   description of what the software does today. It carries no stability promise; a future
   release may change any of it without notice."
2. **"You probably need eight calls, not eighteen"** — the headline. The eight-row table
   from the Ground Truth section above, verbatim, with one sentence saying the other ten
   serve the pre-tournament admin tool and the stream overlay.
3. **Full inventory table** — all 18 operations: method, path, which program calls it,
   whether it needs a token, and the source line. Use the 16 normalised paths so the
   drift check passes.
4. **Empty section headings** for: The refbox eight; Data formats; The other ten;
   Keeping this document honest.

- [ ] **Step 4: Run the drift check to confirm it passes**

Run the Step 1 command.
Expected: `IN SYNC`.

- [ ] **Step 5: Fill in "Keeping this document honest"**

Paste the Step 1 command into that section, with one line explaining that it only
verifies that paths match — never that request or response bodies match — and that the
real check is rebuilding the stub (Task 5).

- [ ] **Step 6: Commit**

```bash
git add docs/third-party-integration.md
git commit -m "docs(workspace): add third-party contract skeleton and endpoint inventory"
```

---

### Task 2: The refbox eight, in full

**Files:**
- Modify: `docs/third-party-integration.md` (the "The refbox eight" section)

**Interfaces:**
- Consumes: the inventory table from Task 1.
- Produces: the only section the stub author in Task 5 is allowed to read.

- [ ] **Step 1: Read every call site**

Read these, in order, and take notes on request shape, response fields actually
consumed, and failure handling:

- `uwh-common/src/uwhportal/mod.rs:206` — `login_to_portal`
- `uwh-common/src/uwhportal/mod.rs:300` — `verify_token`
- `uwh-common/src/uwhportal/mod.rs:537` — `get_event_list`
- `uwh-common/src/uwhportal/mod.rs:501` — `get_event_teams`
- `uwh-common/src/uwhportal/mod.rs:399` — `get_event_schedule_privileged`
- `uwh-common/src/uwhportal/mod.rs:449` — `get_event_referee_name_map_from_referees`
- `uwh-common/src/uwhportal/mod.rs:353` — `post_game_scores`
- `uwh-common/src/uwhportal/mod.rs:325` — `post_game_stats`

- [ ] **Step 2: Document each of the eight to a fixed template**

Every one of the eight gets exactly these headings, so a reader can skim them
side by side:

```markdown
#### N. <Plain-English name>

`METHOD /api/...`  ·  source: `uwh-common/src/uwhportal/mod.rs:NNN`

**When refbox calls it:** <the operator action that triggers it>
**Authentication:** <none | `Authorization: Bearer <token>`>
**Query parameters:** <name, type, meaning — or "none">
**Request body:** <exact JSON, with a worked example — or "none">
**Successful response:** <exact JSON, with a worked example>
**Fields refbox actually reads:** <list; everything else may be omitted>
**On failure:** <status codes and what refbox does>
```

The "Fields refbox actually reads" line is the most valuable line in each entry —
it is what lets a third party return a small object instead of reverse-engineering
the Portal's full response.

- [ ] **Step 3: Give the login flow its own worked example**

Under call 1, write the whole exchange end to end, because it is the one part that
is a conversation rather than a single call:

1. refbox generates a random six-digit number, once per run (`mod.rs:199`).
2. An admin enters that number on the site; the site issues a code.
3. The operator types the code into refbox.
4. refbox posts `{"refBoxId": "<six digits>", "code": "<code>"}`.
5. Success is `200` with `{"accessKey": "<token>"}`.
6. Failure is `400` with `{"reason": "NoPendingLink"}` or `{"reason": "InvalidCode"}` —
   **spelled exactly**, because refbox matches the strings and shows a different
   message for each (`mod.rs:236-248`). Any other reason is reported as an unknown error.

Then add the note that a custom site may skip this entirely: the operator types a
key directly into refbox, and the site only has to accept it as a bearer token.

- [ ] **Step 4: Run the drift check**

Run the Task 1 Step 1 command.
Expected: `IN SYNC` (this task adds no new paths).

- [ ] **Step 5: Commit**

```bash
git add docs/third-party-integration.md
git commit -m "docs(workspace): document the eight calls refbox makes"
```

---

### Task 3: Data formats

**Files:**
- Modify: `docs/third-party-integration.md` (the "Data formats" section)

**Interfaces:**
- Consumes: the eight entries from Task 2, which reference this section rather than
  repeating format details.

- [ ] **Step 1: Document the two ID forms**

Write the verified rule from Ground Truth as a single short subsection, with one
worked example of each form and the three calls that use the long form named.

- [ ] **Step 2: Document the schedule payload**

Read `uwh-common/src/uwhportal/schedule.rs:226` (`Game`), `:241` (`TimingRule`),
`:36` (`ScheduledTeam`), `:210` (`RefereeAssignment`), `:513` (`Schedule`).

Document every field refbox needs to run a game: game number, dark and light teams,
start time, court, timing rule, referee assignments, description. For `TimingRule`,
list all fifteen fields — durations are **whole seconds**, not milliseconds
(`secs_only_duration`). Include one complete worked example of a two-game schedule.

- [ ] **Step 3: Document the timestamp formats**

There are two, and mixing them up is a silent failure:

- Schedule times use `startsOn` in the four-digit-year ISO form
  (`iso8601_4dig_year_no_subsecs`, `schedule.rs:230`).
- Stats events use `occurredOn` in the short-year form
  (`iso8601_short_year`, `refbox/src/tournament_manager/game_stats.rs:14`).

Give a worked example of each, side by side.

- [ ] **Step 4: Document the stats records**

Read `refbox/src/tournament_manager/game_stats.rs:108-152`. Document the three record
kinds — `goal`, `penalty`, `foul` — each tagged with a `$type` field, listing every
field of each with its JSON name (`playerCapNumber`, `gamePeriod`, `periodTime`,
`occurredOn`, `isTotalDismissal`). Include a worked example containing one of each.

State plainly that this is the most Portal-shaped part of the surface, and that a site
that only wants final scores can accept these and discard them — refbox only requires
a `200`.

- [ ] **Step 5: Run the drift check**

Run the Task 1 Step 1 command.
Expected: `IN SYNC`.

- [ ] **Step 6: Commit**

```bash
git add docs/third-party-integration.md
git commit -m "docs(workspace): document ids, timestamps and stats record formats"
```

---

### Task 4: The other ten

**Files:**
- Modify: `docs/third-party-integration.md` (the "The other ten" section)

**Interfaces:**
- Consumes: the template from Task 2 Step 2 — reuse it unchanged.

- [ ] **Step 1: Document the nine schedule-processor calls**

Using the same template as Task 2, from these sources:

- `mod.rs:264` `login_with_email_and_password` — `POST /api/authentication`
- `mod.rs:647` `get_event_schedule_public` — `GET /api/events/{id}/schedule`
- `mod.rs:671` `get_team_roster` — `GET /api/admin/get-event-team` (long-form team ID)
- `mod.rs:726` `get_event_referee_name_map` — `GET /api/events/{id}/participants`
- `mod.rs:772` `get_game_referee_name_map` — `GET /api/admin/events/game-referees`
  (long-form event ID)
- `mod.rs:694` `get_coin_flips` — `GET /api/events/{slug}/schedule/coin-flips`
- `mod.rs:816` `set_coin_flip_result` — `POST` to the same path
- `mod.rs:580` `push_event_schedule` — `POST /api/events/{slug}/schedule`
- `mod.rs:614` `push_team_map` — `POST /api/events/{slug}/schedule/map-teams`

Note in the section heading that none of these are needed to run games — they belong
to the pre-tournament admin tool.

- [ ] **Step 2: Document the one overlay call**

`GET /api/admin/events/{id}/overlay-attachments` — `overlay/src/network.rs:174`.

Add the warning that the overlay makes several calls with its own code rather than
through the shared client (`network.rs:96`, `:240`, `:320`), so a site serving the
overlay must answer those too — they are the same paths as the refbox and
schedule-processor calls, listed here so nobody misses them.

- [ ] **Step 3: Run the drift check**

Run the Task 1 Step 1 command.
Expected: `IN SYNC` — and this is the task most likely to break it, since it adds the
last paths. If it fails, a path in the document is mistyped.

- [ ] **Step 4: Commit**

```bash
git add docs/third-party-integration.md
git commit -m "docs(workspace): document the schedule-processor and overlay calls"
```

---

### Task 5: Build the stub from the document alone

**Files:**
- Create: `docs/third-party-stub/stub_site.py`
- Create: `docs/third-party-stub/README.md`

**Interfaces:**
- Consumes: `docs/third-party-integration.md` — **and nothing else.**

> **The rule that makes this a test:** while writing the stub, do not open any Rust
> source file. If the document does not say something you need, that is a finding.
> Write it down, then go and fix the document — not the stub.

- [ ] **Step 1: Write the stub**

A single Python 3 file using only `http.server` and `json`, serving the eight refbox
calls on port 8099 with a hardcoded fake event: one event, one court, two games, two
teams of six players. Accept any bearer token. Log every request path and body to
stdout so mismatches are visible.

> **No sample code is given here, deliberately.** Every other task in this plan shows
> the exact content to produce; this one must not. Handing over a working stub would
> mean the stub was written from the plan rather than from the document, and the test
> would prove nothing. The request and response shapes are in
> `docs/third-party-integration.md` — if they are not, that is the bug this task exists
> to find.

- [ ] **Step 2: Start the stub**

```bash
python3 docs/third-party-stub/stub_site.py
```
Expected: `serving on http://localhost:8099`.

- [ ] **Step 3: Run refbox against it**

```bash
UWH_PORTAL_URL_OVERRIDE=http://localhost:8099 cargo run -p refbox -- --allow-http
```

In the app: open settings, turn the Portal on, link with any code the stub accepts,
then select the event, the court, and a game.

Expected: the event appears in the list, the court appears, the game appears with both
team names.

- [ ] **Step 4: Record every documentation bug**

For each thing that did not work, write down what the document failed to say, then fix
`docs/third-party-integration.md` and repeat Step 3 until selection works end to end.

Keep the list — it goes in the commit message, because it is the evidence that this
task did its job. A run that finds **zero** documentation bugs should be treated as
suspicious, not as success: check that the stub was genuinely written from the document.

- [ ] **Step 5: Write the stub README**

`docs/third-party-stub/README.md`: what the stub is for, the two commands above, that
it is not part of the cargo workspace and never runs in CI, and that its only purpose
is to prove the document is complete.

- [ ] **Step 6: Commit**

```bash
git add docs/third-party-stub/ docs/third-party-integration.md
git commit -m "docs(workspace): add verification stub and fix contract gaps it found"
```

---

### Task 6: Run a full game through the stub

**Files:**
- Modify: `docs/third-party-stub/stub_site.py`, `docs/third-party-integration.md`

**Interfaces:**
- Consumes: the stub from Task 5.

- [ ] **Step 1: Run a complete game**

With the stub running and a game selected, play a game through to the end: start the
clock, score for both teams, add a penalty and a foul, and end the game.

- [ ] **Step 2: Confirm the score arrives**

Expected in the stub's log: `POST /api/events/<id>/schedule/games/<n>/scores` with a
body of the documented shape, and the scores matching what was entered.

- [ ] **Step 3: Confirm the stats arrive**

Expected in the stub's log: `POST /api/admin/events/stats` with `eventId` in the
**long** form and `gameNumber` as query parameters, and a body containing the goal,
penalty and foul records in the documented `$type` shape.

- [ ] **Step 4: Confirm the health indicator goes green**

Expected: refbox's portal indicator shows the result as sent, not queued or stuck.
If it stays queued, the stub returned something refbox did not accept — that is a
documentation bug about the expected response, and the most likely one in the whole
exercise.

- [ ] **Step 5: Fix the document and repeat**

Repeat Steps 1–4 until a full game flows through with no manual intervention.

- [ ] **Step 6: Add the worked transcript to the document**

Append the actual logged requests and responses from a clean run to
`docs/third-party-integration.md` as a "Worked example: one complete game" section.
A real transcript is worth more than any amount of prose.

- [ ] **Step 7: Commit**

```bash
git add docs/third-party-stub/ docs/third-party-integration.md
git commit -m "docs(workspace): verify contract with a full game against the stub"
```

---

## Definition of done

- `docs/third-party-integration.md` documents all 18 operations, each citing its source line.
- The drift check reports `IN SYNC`.
- A stub written from the document alone runs a complete game — selection, scores,
  stats — with refbox's health indicator green.
- The document carries the v0.4.9 accuracy marker and an explicit best-effort disclaimer.
- No Rust source file has been modified.

## Explicitly not in this plan

- The refbox data-source picker and the typed-entry page (spec Sections 1–2).
- The schedule-processor "Other…" menu entry and the overlay documentation section
  (spec Section 3, deliverable three).
- Any change to the 18 calls themselves, the retry queue, or the health indicator.

## Deviations

_(Record here if execution diverges from the plan.)_

- **Task 1, Step 1 command fixed:** the original drift check used `/api/[A-Za-z0-9/{}_-]*`
  (a `*` quantifier). Once the check's own text was pasted verbatim into the document's
  "Keeping this document honest" section (Step 5), that literal text self-matched: `/api/`
  followed by `[` (not in the character class) still counts as a zero-length match under
  `*`, producing one spurious `/api/` entry on the document side with no counterpart in the
  source files. Changed the quantifier to `+` in both extractions (code side and doc side),
  which requires at least one trailing path character and no longer matches the check's own
  quoted regex text. Verified against the final committed document: `IN SYNC`, no extra
  lines. Tasks 2, 3, and 4 should use the `+` version when they re-run this check.
