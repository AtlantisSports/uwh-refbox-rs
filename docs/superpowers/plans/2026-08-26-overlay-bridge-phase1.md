# Overlay Bridge (phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> **Plan style:** this repository's `.claude/rules/plan-execution.md` overrides the writing-plans
> skill's default granularity. This is a rough task list with concrete acceptance points, not a
> line-by-line script. Executors are expected to make reasonable decisions inside the sketch.

**Goal:** A program that runs on the streaming PC, reads the refbox's existing network feed, and
serves the live game as JSON tables that vMix (or anything else) can poll — surviving refbox
dropouts without the on-screen picture ever going wrong.

**Architecture:** Async Rust on tokio. A feed reader consumes newline-delimited snapshots from the
refbox and supervises its own connection. A state holder decides whether the clock was running and
continues it locally when that is correct. A Portal client resolves cap numbers to player names and
never treats a failure as fatal. An axum server exposes fixed-length JSON tables plus an operator
status page.

**Tech Stack:** Rust 2024 · MSRV 1.85 · tokio · serde / serde_json · reqwest · **axum (new
dependency, approved by Eric 2026-08-26)** · uwh-common (for `GameSnapshot`)

**Spec:** `docs/superpowers/specs/2026-08-26-vmix-overlay-bridge-design.md`
**Companion:** `docs/superpowers/specs/2026-08-26-vmix-integration-steps.md`

---

## Global Constraints

Every task's requirements implicitly include this section.

- **Rust edition 2024, MSRV 1.85.** No language or standard-library features newer than 1.85.
- **`cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass** on Linux,
  Windows and macOS. Zero warnings. No `#[allow(...)]` without discussion.
- **CI's actual gate is weaker than that**, and the difference matters. `.github/workflows/rust.yml`
  runs `cargo clippy --all -- --deny=warnings` (and a `--no-default-features` pass) — **without
  `--all-targets`**. Consequence: any module compiled only under `cfg(test)` is **not linted by CI
  at all**. Never hide a module behind `#[cfg(test)]` to satisfy the dead-code lint; that trades a
  warning for a silent coverage hole.
- **The crate is a library plus a thin binary** (`src/lib.rs` + `src/main.rs`). Unused `pub` items
  in a library are legitimately part of its API surface and are exempt from `dead_code`, so a module
  that no later task has wired up yet still compiles and lints normally. This is the structural
  answer to the problem above — do not reintroduce per-module gates.
- **`cargo fmt --all` before every commit.**
- **No `unwrap()` or `expect()` in non-test code** without a comment explaining why it cannot panic.
- **Do NOT modify `refbox`, `uwh-common`, `overlay`, or the shared data format.** This is the
  central design decision (spec §4.2): the bridge must work with every refbox already in the field.
  A task that seems to need a refbox change has gone wrong — stop and raise it.
- **Default HTTP port is 8099. Never 8088** — that is vMix's own web controller and would collide.
- **Lost-contact threshold is at least 3 seconds.** Measured: the tick is 1.000s while running, but
  operator interaction stretches gaps to 1.5–2.0s. A shorter threshold flashes red spuriously.
- **Every served table is a JSON array of objects, one object per row**, fixed length, blank-padded.
  vMix requires the array form and binds titles to explicit row numbers.
- **Every served value is a string**, including numbers.
- **Column names are a published contract.** Choose once; renaming later breaks every title built
  against them.
- **Read the feed to the newline. Never into a fixed-size buffer.** This is the exact defect that
  breaks the current overlay (spec §10.1).
- Approval gates from `.claude/rules/communication.md` apply: ask before branches, commits, pushes.

---

## File structure

New workspace member `overlay-bridge/`. Added to the `members` list in the root `Cargo.toml`.

| File | Responsibility |
|---|---|
| `src/main.rs` | CLI parsing, config load, wiring, tokio runtime. Nothing else. |
| `src/feed.rs` | Newline-framed snapshot reader, and the connection supervisor that owns keepalive and reconnection. |
| `src/state.rs` | The live picture: which snapshot is current, whether the clock was running, the locally-continued clock, and contact status. No I/O. |
| `src/portal.rs` | Schedule and team lookups. Caching, retry, never fatal. |
| `src/tables.rs` | The served table shapes. Pure transformation from state to rows. No I/O. |
| `src/server.rs` | axum routes and their wiring to `tables`. |
| `src/status.rs` | The operator status page. |
| `src/config.rs` | Persisted settings: last refbox address, side of pool, court. |
| `src/discovery.rs` | Local-network scan for refboxes, and confirming that something is one. |

`state.rs` and `tables.rs` deliberately contain no I/O — they hold the logic most worth testing, and
they must be testable without a refbox, a network, or a clock.

---

## Task 1 — Crate skeleton and the newline-framed feed reader

**Files:** create `overlay-bridge/Cargo.toml`, `src/main.rs`, `src/feed.rs`; modify root
`Cargo.toml` (add to `members`).

**Produces:** `feed::SnapshotReader::new(impl AsyncRead)` yielding
`impl Stream<Item = Result<GameSnapshot, FeedError>>`.

Reads to the newline. Nothing about buffer sizes appears anywhere in this file.

**Tests must prove:**
- One message per newline is parsed.
- **Several messages arriving in a single read are all parsed** — not just the first.
- **A message split across two reads is parsed once, whole.**
- **A message far larger than any plausible buffer (say 8 KB) is parsed correctly.** This is the
  regression guard for the defect that breaks the overlay; it must exist from day one.
- A malformed line is reported and skipped **without** desynchronising the following messages.
- A truncated final line at end-of-stream is not reported as a parse failure.

Fixtures: real captured messages are in the phase-0b capture. Use at least one genuine 794-byte
message with five penalties, a foul and a warning, including a `TotalDismissal`.

**Wire-format facts the parser must tolerate** (all observed on a live feed, 2026-08-26):
- A penalty's `time` is **either** `{"Seconds": 25}` **or** the string `"TotalDismissal"`.
- The fouls bundle has **three** keys: `black`, `equal`, `white`. `equal` is the both-at-fault bucket.
- `game_number` and `next_game_number` are **strings** (`"1"`), not numbers.
- `recent_goal` is a two-element array: `["Black", 6]`.
- `event_id` may be `null`.
- `timeout` is `null` or an object such as `{"Black": 30}`.

Deserialising into `uwh_common::game_snapshot::GameSnapshot` handles all of this. Do not hand-roll a
parallel type.

**Commit:** `feat(overlay-bridge): add crate and newline-framed feed reader`

---

## Task 2 — The state holder: clock continuation and contact status

**Files:** create `src/state.rs`.

**Consumes:** `GameSnapshot` from Task 1.
**Produces:** `state::LiveState` with `apply(snapshot, at: Instant)`,
`current(now: Instant) -> Display`, and `contact(now: Instant) -> Contact`.

`Display` carries the clock the bridge believes is correct right now. `Contact` is
`Live | Stale { since: Duration }`.

**The rule, from spec §5.4** — derived entirely on this side, with no help from the refbox:

- Updates were arriving and then stopped → the clock **was running** → **keep counting locally.**
- Updates were not arriving → the clock **was stopped** → **hold everything.**

"Was arriving" means at least two snapshots whose arrival times were under the threshold apart and
whose `secs_in_period` differed.

**Time must be injected.** Every function takes `now: Instant`. **No non-test code** in this plan
may call `Instant::now()` inside `state.rs` — tests drive the clock by hand, and a test that sleeps
is a failed test. Tests may seed a single `t0 = Instant::now()` and derive every later timestamp
by arithmetic (`t0 + Duration::from_secs(n)`); `std::time::Instant` has no other constructor, so
that is not a violation.

**Tests must prove:**
- Ticks arriving each second, then silence → the clock continues to count down in real time.
- No ticks at all, then silence → the clock holds at its last value indefinitely.
- **Sudden death counts UP, not down.** (`GamePeriod::SuddenDeath` inverts the direction —
  `uwh-common/src/game_snapshot.rs:203`.) A locally continued sudden-death clock that counts down
  is a bug this test must catch.
- A 2.0-second gap during normal running does **not** report `Stale`. (Measured reality: operator
  interaction stretches gaps this far.)
- A 4-second gap while the clock was running **does** report `Stale`, with a plausible duration.
- The moment a real snapshot arrives, it **overwrites** the locally continued value, even if the
  local estimate had drifted.
- The clock never counts below zero, and never past the end of a period into a negative.

**Commit:** `feat(overlay-bridge): continue the clock locally through dropouts`

---

## Task 3 — Connection supervisor: connect, keepalive, reconnect

**Files:** modify `src/feed.rs`.

**Produces:** `feed::Supervisor::run(addr, tx)` — connects, streams snapshots into `tx`, and
reconnects on loss, retrying every second.

**TCP keepalive is mandatory, and it is the point of this task.** Spec §10.4: because the feed is
one-way, the bridge never transmits, so a refbox that has silently gone away is never detected and
the read waits forever. That is the most likely cause of the overlay freezes seen at real events.
**A bridge without keepalive inherits that exact bug.**

**Use `socket2`** — decided by Eric, 2026-08-26. It exposes the per-socket keepalive settings on
both Linux and Windows, and it is **already in the workspace dependency tree** via tokio, so this
adds a feature flag rather than a new library. Configure keepalive so a dead peer is noticed within
roughly ten to fifteen seconds.

A read timeout was considered and **rejected**: silence is legitimate here — a 25-second
stopped-clock silence was measured on 2026-08-26 — so any timeout short enough to detect a dead
refbox quickly would also fire during a normal half-time. That confuses the two meanings of
silence, which is the exact mistake this design exists to avoid.

**Carried from Task 1's review:** `feed.rs`'s `line_buf` grows without a cap if a peer connects and
never sends a newline. That is correct for Task 1 (no size ceilings), but peer misbehaviour is this
task's business. Decide whether the supervisor should drop a connection that has sent a very large
amount with no newline, and say what you decided either way.

**Tests must prove:**
- Snapshots arriving on a stream reach the channel in order.
- A closed stream triggers a reconnect attempt rather than ending the task.
- Connecting to a refused address retries rather than exiting.
- The keepalive settings are actually applied to the socket (assert on the socket options, not on
  behaviour — a real half-open connection cannot be produced reliably in a unit test).

**Commit:** `feat(overlay-bridge): supervise the feed connection with keepalive`

---

## Task 4 — Portal client: team names and rosters, never fatal

**Files:** create `src/portal.rs`.

**Produces:** `portal::Directory` with `names_for(game_number) -> Option<TeamNames>` and
`player_name(team, cap_number) -> Option<String>`.

**No credentials anywhere** — verified: the overlay sends none, and both calls below were made
unauthenticated against the live dev portal on 2026-08-26 to capture the fixtures.

**One call gets both team names.** `GET {portal}/api/events/{event_id_partial}/schedule` returns:

- `games` — an **array**. Find the game whose `number` matches (it is a **string**, e.g. `"1"`).
- **`court` and `startsOn` live on the matched game, never at the top level.** Reading them from
  the top level was PR #2474's bug; do not reintroduce it.
- `dark.assignment.teamId` / `light.assignment.teamId` — e.g. `"teams/2529-B"`, or **`null`** when
  the slot is not yet assigned (a bracket placeholder). Handle null; it is common.
- **`teams`** — a top-level **object keyed by team id**, each with a `name` (and a `logo`). This is
  where team names come from. **A second call is not needed for names.**

**The second call is only for player rosters** (turning a cap number into a name):
`GET {portal}/api/admin/get-event-team?teamId={team_id_full}` returns `name`, `logoUrl`, `photos`
and `roster`. **Each roster member's fields are `capNumber`, `rosterName`, `roles`, `photos` —
not `number`, `name`, `role`.** An earlier draft of this plan named the wrong three; the overlay
itself reads the correct ones (`overlay/src/network.rs:126-136`).

Follow the overlay's two conventions for display: a team name is trimmed and upper-cased, and a
member with no `rosterName` displays as `"Player"`.

**Fixtures are already captured and committed** — use them, do not invent responses:
- `overlay-bridge/tests/fixtures/schedule-response.json` — trimmed from the real dev-portal
  response for event `1889-B`: two games (one with both teams assigned, one with `teamId: null`)
  and the `teams` entries they reference.
- `overlay-bridge/tests/fixtures/team-roster-response.json` — a real 12-member roster response.
  Cap numbers and every field name are exactly as the portal returned them; **the player names are
  replaced and photo URLs nulled**, because the originals are real people and this file is
  committed.

**No failure is ever fatal.** Spec §5.5 and §10.2: the overlay treats a failed team fetch as final
for that lookup and never retries, so a brief Portal outage means a game's names never appear. The
bridge does the opposite: retry on a timer, cache every success, keep serving the last good copy,
and if a roster has never been fetched, serve cap numbers with empty names.

**No `expect()` on any network call, response body, or JSON parse.** Every one of those is a
panic point in the overlay today.

**Tests must prove:**
- The schedule fixture yields the right two team IDs, court and start time for game `"1"`, and the
  team names come from the top-level `teams` map with no second call.
- The game with `teamId: null` yields no team id and does not error — the bridge shows whatever the
  refbox gave it rather than failing.
- A roster fixture maps cap numbers to names.
- A failing request leaves a previously cached roster intact and still served.
- A failing request with no cache yields empty names, not an error and not a panic.
- A malformed JSON body is handled without panicking.

**Commit:** `feat(overlay-bridge): resolve team and player names from the portal`

---

## Task 5 — The served table shapes

**Files:** create `src/tables.rs`.

**Consumes:** `state::Display`, `portal::Directory`.
**Produces:** `tables::scorebug()`, `tables::penalties()`, `tables::fouls()`, `tables::warnings()`,
`tables::next_game()` — each returning `Vec<BTreeMap<String, String>>`.

Shapes are specified in the companion document; follow it exactly, because those column names
become a published contract.

**Rules:**
- Every table is a **fixed length**, padded with rows whose every value is `""`. A vMix title bound
  to row 3 must always find a row 3.
- Every value is a **string**. Times appear twice: display-ready (`"3:47"`, `"1:42"`, `"TD"`) and as
  a plain number of seconds (`"227"`).
- `/scorebug` is always exactly one row.
- Penalty, foul and warning tables have a fixed row count. Start at six and **check the real ceiling
  against what a game can produce** before fixing it; record the chosen number and the reasoning.
- Which team is on the left is an operator setting — the feed does not carry it
  (`refbox/src/app/update_sender.rs:536-545`).

**Tests must prove:**
- With no penalties, `/penalties` still returns the full row count, every value empty.
- With two penalties, rows 1 and 2 are populated and the rest are empty.
- A `TotalDismissal` renders as `"TD"` in the display column, and its seconds column is empty
  rather than `"0"` (a dismissal has no countdown).
- A cap number with no roster entry renders the number with an empty name, never `"None"`,
  `"null"`, or `"Unknown"`.
- Swapping the side-of-pool setting swaps which team appears in the left-hand columns.
- The both-at-fault (`equal`) foul bucket appears in the fouls table and is not silently dropped.

**Commit:** `feat(overlay-bridge): serve the game as fixed-length tables`

---

## Task 6 — The web server

**Files:** create `src/server.rs`; modify `src/main.rs`.

**Produces:** an axum app serving `GET /scorebug`, `/penalties`, `/fouls`, `/warnings`,
`/nextgame`, and `/status.json`, on port 8099 by default and configurable.

Every response recomputes the clock at request time from `state::LiveState`, so what vMix reads is
correct to the moment rather than correct as of the refbox's last message.

**Tests must prove:**
- Each route returns HTTP 200, `Content-Type: application/json`, and a JSON **array**.
- Two requests a second apart against a running clock return **different** clock values, proving
  the recompute-per-request behaviour rather than a cached body.
- The port is configurable and defaults to 8099.
- An unknown path returns 404 rather than panicking.

**Commit:** `feat(overlay-bridge): serve the tables over http`

---

## Task 7 — Config and the operator status page

**Files:** create `src/config.rs`, `src/status.rs`.

**Produces:** persisted settings (last refbox address, side of pool, court, port) and `GET /`
serving the status page.

The status page is available **the moment the bridge starts, before any refbox is configured** —
there must be no chicken-and-egg. It shows a large green/red indicator, time since last contact,
the current event/game/period, the manual address field, the two operator settings, and the
addresses to paste into vMix.

**It must also report whether the connection check (TCP keepalive) is actually active.** Task 3
configures it, but if the operating system or network stack refuses, the supervisor logs to stderr
and carries on reading. That is the right call — tearing down a connection that still delivers
frames would turn degraded detection into a total outage — but stderr is invisible to an operator
running a compiled program that feeds vMix, and the bridge would then be silently back to the
freeze behaviour Task 3 exists to prevent. **This task owns making it visible, and owns adding
whatever minimal signal the supervisor must expose** — Task 3 deliberately did not build a flag
with no reader. Wording along the lines of "connection check unavailable — a lost refbox may not
be detected". Rare on Windows and Linux in practice; this is insurance, not a common case.

**Tests must prove:**
- Settings round-trip through save and load.
- A missing or corrupt settings file yields defaults rather than an error.
- `/` returns HTML with a 200 even when no refbox has ever been reached.
- `/status.json` reports `Stale` with a duration once contact is lost, and `Live` otherwise.
- The page reports the connection check as unavailable when the supervisor could not enable it, and
  as active when it could.

**Commit:** `feat(overlay-bridge): add settings and the operator status page`

---

## Task 8 — Refbox discovery

**Files:** create `src/discovery.rs`; modify `src/status.rs`.

**Produces:** `discovery::scan(subnet, port) -> Vec<Found>` where `Found` carries the address and a
human label built from the first snapshot the box sends.

Discovery works because **a refbox replays its current state the instant anything connects**
(`refbox/src/app/update_sender.rs:606-630`). So a candidate is confirmed to be a refbox — not merely
an open port — by connecting, reading one snapshot, and labelling it:

```
192.168.1.50   Game 14 · Second Half · 3:47 · 2–1
192.168.1.51   Game 15 · Between Games
```

Scan concurrently with a short timeout, and close each probe as soon as one snapshot is read.

**Tests must prove:**
- A fake server that sends a valid snapshot on connect is reported, with the right label.
- A port that accepts but sends nothing is **not** reported (it is not a refbox).
- A refused port is not reported and does not fail the scan.
- The scan completes within a few seconds for a full 254-address range.

Manual entry stays available: a first scan may raise a Windows firewall prompt, and some venue
networks block it.

**Commit:** `feat(overlay-bridge): find refboxes on the local network`

---

## Task 9 — Windows build and packaging

**Files:** modify `justfile`; possibly `.github/workflows/`.

Follow the existing `schedule-processor` Windows cross-compilation precedent — do not invent a new
approach. **Any CI workflow change is shared infrastructure: ask before making it.**

**Deliverable:** a Windows executable that runs on the streaming PC and serves its status page.

**Commit:** `chore(overlay-bridge): build for windows`

---

## Acceptance

The walkthrough Eric runs, from spec §8. Steps 7 and 8 are the whole design in two actions:

1. Start the bridge; open its status page.
2. It lists refboxes found, each labelled with the game on it. Pick the court.
3. Green indicator, "last heard from: just now", vMix addresses shown.
4. In vMix, add a Data Source pointing at `/scorebug`; bind a title to team names, scores, clock.
5. Score a goal on the refbox — the bug updates.
6. Issue a penalty — it appears with a countdown and the player's name.
7. **Cut the refbox's network mid-half** — the clock keeps counting on screen; the status page goes
   red with time since contact. Restore it — the clock corrects silently.
8. **Stop the clock on the refbox normally** — the clock holds; the status page stays green.

**Where each test can run** (spec §8.1): the Windows host reaches services inside WSL, measured — so
the vMix leg needs no second PC. Only step 7 needs real hardware, because loopback never drops.

**Do not claim completion without running these.** `just check` passing is not evidence that the
bridge works.

---

## Out of scope for phase 1

- vMix title design (phase 2 — Eric or a titles builder).
- The third-party feed contract document (phase 3, deliberately written last).
- Both overlay defects — recorded in `docs/backlog/overlay-feed-reader-defects/NOTE.md`. Neither
  blocks this work; the bridge inherits neither by construction.
- Images: logos, flags and player photographs. Text only in phase 1.
- Any change to the refbox, uwh-common, or the shared data format.

---

## Deviations

Record here as they happen, per `.claude/rules/plan-execution.md`. Do not create standalone
deviation commits — fold the note into the code commit that caused it.
