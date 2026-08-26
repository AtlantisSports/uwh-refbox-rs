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
- **Never infer anything from silence.** The refbox sends nothing at all while the clock is stopped
  (25 seconds measured), so message timing can never indicate a lost connection. Liveness comes only
  from the connection itself — read error, end-of-stream, or keepalive failure. (This supersedes an
  earlier "lost-contact threshold of at least 3 seconds"; see Task 10.)
- **The bridge never invents a value.** It serves what the refbox sent, or nothing. No projection,
  no interpolation, no derived clock.
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

**Court and start time come from `portal::TeamNames`,** which Task 4 now exposes as `court:
Option<String>` and `start_time: Option<String>` — the latter a **raw ISO 8601 string** exactly as
the Portal returned it (e.g. `"2026-08-01T09:30:00+10:00"`). Render it as `HH:MM` using the offset
carried in the timestamp, which is what the overlay does (`overlay/src/network.rs:16,339-341` —
format `[hour]:[minute]`, giving `09:30`). The `time` crate is already a workspace dependency.

**Serve bare values, never baked-in labels.** The overlay emits `"COURT: 2"` and `"START: 09:55"`
(`overlay/src/network.rs:335,340`) because it draws the picture itself. **The bridge must not.**
vMix titles add their own prefix through the data source's Format setting (`Court {0}`), so a label
baked into the value cannot be removed — the operator would get `Court COURT: 2` and have no way to
fix it short of us changing the code. Serve `"2"` and `"09:30"`.

**Tests must prove:**
- With no penalties, `/penalties` still returns the full row count, every value empty.
- With two penalties, rows 1 and 2 are populated and the rest are empty.
- A `TotalDismissal` renders as `"TD"` in the display column, and its seconds column is empty
  rather than `"0"` (a dismissal has no countdown).
- A cap number with no roster entry renders the number with an empty name, never `"None"`,
  `"null"`, or `"Unknown"`.
- Swapping the side-of-pool setting swaps which team appears in the left-hand columns.
- The next-game row renders a raw ISO 8601 start time as `HH:MM`, and serves the court and time as
  bare values with no `COURT:`/`START:` prefix baked in.
- A game with no court or no start time yields empty strings, not `"None"` or a placeholder.
- The both-at-fault (`equal`) foul bucket appears in the fouls table and is not silently dropped.

**Commit:** `feat(overlay-bridge): serve the game as fixed-length tables`

---

## Task 6 — The web server

**Files:** create `src/server.rs`; modify `src/main.rs`.

**Produces:** an axum app serving `GET /scorebug`, `/penalties`, `/fouls`, `/warnings`,
`/nextgame`, and `/status.json`, on port 8099 by default and configurable.

Every response recomputes the clock at request time from `state::LiveState`, so what vMix reads is
correct to the moment rather than correct as of the refbox's last message.

**This task also owns the Portal refresh loop, and nothing else does.** Task 4 built
`portal::Directory`'s refresh methods to be safe to call repeatedly, but deliberately did not drive
them: `tables.rs` is pure with no I/O and cannot own a timer, and no other task touched it. So the
loop falls to the wiring here — `main.rs`, whose charter is exactly CLI, config, wiring and the
runtime. Spawn a periodic refresh alongside the feed supervisor, the same way `Supervisor::run` is
already spawned, and refresh when the game number changes. **"Retry on a timer" is the whole reason
Task 4 was specified as never-fatal; without this loop a Portal outage is permanent for that run.**

**It also decides which game is "next".** Task 5's `next_game()` deliberately does not — it renders
whatever game number it is handed. The answer is already in the feed: every snapshot carries
`next_game_number` alongside `game_number` (`uwh-common/src/game_snapshot.rs:51-52`), so the wiring
passes that through. Do not invent a rule for it.

**Tests must prove:**
- Each route returns HTTP 200, `Content-Type: application/json`, and a JSON **array**.
- The Portal refresh loop retries after a failure rather than giving up, and a refresh failure never
  disturbs the game data coming from the refbox.
- `/nextgame` renders the game the feed names in `next_game_number`, not the current game.
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
there must be no chicken-and-egg. It shows a large green/red indicator, how long the connection has
been down (only while it is down), the current event/game/period, the manual address field, the two
operator settings, and the addresses to paste into vMix.

**Amended after Task 10 — read this before writing any test.** Task 10 deleted `state::Contact` and
its `Live`/`Stale { since }` pair outright, because both were derived from how long it had been
since a message ARRIVED. Do not reinstate them or anything shaped like them. The refbox sends
nothing at all whenever the clock is stopped (25 seconds measured), so a page driven by message
timing would show red at every stoppage. Everything on this page that concerns liveness comes from
the connection: `feed::Connection`, which distinguishes never-connected, connected, and
disconnected. The only duration this page may show is **time since the connection dropped**, and
only while it is dropped — never time since the last message. Never-connected shows no duration at
all, because there is nothing to measure from.

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
- `/status.json` distinguishes all three connection states — never connected, connected, and
  disconnected — and carries a duration only in the disconnected case.
- **The regression guard for the trap, on this surface:** with the connection alive and no messages
  arriving for well over any plausible timeout, the page and `/status.json` still report connected
  and show no duration. This must fail against any implementation that derives liveness from
  message timing. `server.rs`'s existing `assert_scorebug_survives_silence` helper is the pattern to
  copy, including its 30-second case.
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

**Amended after Task 7 — this task OWNS runtime address selection, both halves of it.** The earlier
wording said "manual entry stays available", which assumed Task 7 had shipped an editable address
field. It did not: Task 7's page displays the address read-only, and changing it means relaunching
the program with a flag. That was the right call there — a field that saves an address without
reconnecting is worse than no field — but it means the machinery has no owner until here.

So this task builds both ways of choosing a refbox, because both need the same thing underneath:
**setting the address at runtime and making the supervisor reconnect to it.** Picking one from the
scan results and typing one by hand are two front ends onto that single mechanism. Manual entry is
not optional garnish — a first scan may raise a Windows firewall prompt, and some venue networks
block scanning entirely, so typing an address must always work.

Reconnecting must not weaken the connection rule: the supervisor drops its current connection and
connects to the new address, and liveness continues to come from the connection itself, never from
message timing. Serving the last values from the old refbox while the new one is being reached
would be exactly the "confidently wrong" behaviour §4.6 removed.

**Additional tests must prove:**
- Setting a new address makes the supervisor connect to it, and the tables then serve that
  refbox's game rather than the previous one's.
- A submitted address that is malformed or unreachable is reported to the operator and leaves the
  existing connection alone, rather than tearing down a working one.
- The chosen address persists, so a restart comes back to the same refbox.
- **Carried from Task 7, and it becomes reachable here:** `feed.rs`'s `set_disconnected()` holds a
  guard that must not reset `disconnected_at` when the connection is *already* disconnected. Task 7
  left it untested because nothing called it twice. Changing address while already disconnected is
  exactly that second call, so prove the guard: the "down for" time must keep counting from the
  original drop, not restart.

**Commit:** `feat(overlay-bridge): find refboxes on the local network`

---

## Task 9 — Windows build and packaging

**Files:** modify `justfile`; possibly `.github/workflows/`.

Follow the existing `schedule-processor` Windows cross-compilation precedent — do not invent a new
approach. **Any CI workflow change is shared infrastructure: ask before making it.**

**Deliverable:** a Windows executable that runs on the streaming PC and serves its status page.

**Commit:** `chore(overlay-bridge): build for windows`

---

## Task 10 — Relay only: stop inventing values, hide the graphic when disconnected

**Execute this NEXT, before Tasks 7–9.** It reverses part of Tasks 2, 3, 5 and 6, all of which are
complete and review-clean. Numbered 10 only to keep earlier task indices stable.

**Origin:** Eric, 2026-08-26, on seeing the complexity the clock projection caused — "I will want
the overlay to turn off if it loses connection, instead of displaying wrong info or guessed
(calculated) info." Full reasoning in the spec, §4.6 (reversed) and §5.4 (rewritten).

**Files:** modify `src/state.rs` (mostly deletion), `src/feed.rs`, `src/tables.rs`, `src/server.rs`,
and `docs/superpowers/specs/2026-08-26-vmix-integration-steps.md`.

### Delete

From `state.rs`: the clock projection, the counting-direction handling, and the inference of whether
the clock was running. `LiveState` keeps the last snapshot and when it arrived — nothing more. The
`started`-flag seeding problem disappears with it. **This is mostly deletion; resist the urge to
replace it with something.**

### Add

`feed.rs` publishes **connection state** — connected or not — updated when the supervisor connects,
when a read fails, when the stream ends, and when keepalive reports the peer gone. The server reads
it. **Liveness must come from the connection, never from message timing.**

`tables.rs` gains a **`connected`** column on **every** table (a penalties title binds to
`/penalties` and needs the flag on the source it reads). When disconnected, every other value in
every row is **blanked**, so a title never wired to the flag degrades to empty text rather than
stale numbers. Eric's preference is the flag as the primary mechanism, with blanking as the
backstop.

`server.rs` wires it: handlers ask the feed for connection state and pass it to the table builders.

**Also add the both-at-fault foul total to `/scorebug`** (Eric, 2026-08-26 — this closes the
question Task 5's review left open). The scorebug already carries per-team foul totals for black
and white; it gains a third, independent count for equal ("both at fault") fouls. *Why:* `/fouls`
already lists equal fouls as their own rows, so two totals under-report against the list printed
beside them, which reads as a defect on air. Folded in here only because Task 10 reopens
`tables.rs` anyway — it is otherwise unrelated to the relay-only change. Like the per-team totals,
it counts every entry, not only those within the rows carried.

### The trap

**Do not hide the graphic on silence.** The refbox sends nothing whenever the clock is stopped — 25
seconds observed — so a silence-based rule would blank the graphic every time the referee stops the
clock. Connection alive plus no messages means *the clock is stopped*, and the last message is then
exactly right, not stale.

### Tests must prove

- With the connection alive and no messages arriving for well over any plausible timeout, the tables
  still serve the last real values and `connected` stays true. **This is the regression guard for
  the trap above** — it must fail against a silence-based implementation.
- On disconnection, `connected` is false on every table and every other value is blank.
- On reconnection, values return.
- No served clock value ever differs from the last one the refbox sent — assert directly that the
  clock is relayed verbatim, since removing projection is the whole point.
- Sudden death relays verbatim like any other period. (The old direction-handling test should be
  deleted, not adapted.)
- The both-at-fault foul total is independent of the two per-team totals — a game with equal fouls
  and no team fouls must show the equal count non-zero while both team counts stay zero — and, like
  them, counts entries beyond the rows carried.

**Commit:** `refactor(overlay-bridge): relay only, hide the graphic when disconnected`

---

## Task 11 — RESOLVED, NOT A CODE DEFECT: security software was blocking the Windows build

**Closed 2026-08-27 without any code change.** Recorded in full because the wrong conclusion was
one step away, and because the operational half of it is still open.

**What was seen.** Task 9 — required to prove the Windows executable actually *serves* rather than
merely build — found that `overlay-bridge.exe` started, ran as a genuine Windows process, accepted
a TCP connection on its HTTP port, and never answered. Confirmed from native Windows loopback with
three independent clients over 45 seconds.

**What it actually was.** Eric's security software had blocked the executable, flagging it
`IDP.Generic` — a behavioural heuristic, not a match for any known threat. After he restored the
file and added an exception, the same binary served immediately: `GET /` returned HTTP 200 in
0.51 s, `/scorebug` and `/status.json` both served correctly, with `connected: "false"`, every
other column blank, and `contact: "NeverConnected"` carrying no duration — the relay-only and
three-state connection behaviour working on Windows exactly as designed.

**Why the evidence pointed the wrong way, and the lesson.** The control that seemed to exonerate
the environment — a minimal axum app built the identical way, which served instantly — was *also*
flagged by the same software, which Eric discovered only afterwards. It had served during the test
and been quarantined later. A control is only a control while it is running under the same
conditions as the thing it is controlling for, and that cannot be assumed when an external agent
is quietly acting on both.

**Why the heuristic fires, which is not a mystery.** The binary is unsigned, freshly compiled, and
Task 8 gave it a sweep of 254 addresses on the local network to find refboxes. From the outside
that is indistinguishable from network reconnaissance. This is the expected reaction to what the
program legitimately does, not a fluke.

**DECIDED, Eric, 2026-08-27: ship it exactly like refbox.** Build Windows, Mac ARM and Mac Intel
versions as release assets and let the operator bypass the security warning manually, as they
already do for refbox. No code signing for now.

This is a better answer than the options offered against it, for a reason worth recording: the
release workflow builds each platform **natively on that platform's runner** — Windows on
`windows-latest`, producing an MSVC-toolchain `refbox.exe`, not a mingw cross-build. So following
the existing pattern removes the mingw factor for free, without anyone having to decide about it.
The binary blocked on 2026-08-27 was the worst possible case: cross-compiled with mingw, built
locally, downloaded by nobody, and therefore carrying no reputation at all. What an operator
downloads from a release is not that file.

Carried into phase 2 as consequences, not blockers:
- **Antivirus quarantine is a harder failure than SmartScreen's "Run anyway".** Eric hit one file
  he could not create an exception for at all. A locked-down or managed laptop may simply refuse,
  so the operator documentation needs a fallback — running the bridge on another machine on the
  venue network, which works because vMix only needs a URL.
- **macOS is stricter than Windows, and the shape is an open question.** refbox ships `.app`
  bundles via `cargo bundle` because it is a GUI application. overlay-bridge is a headless server
  with a web page: a plain binary would require a volunteer to use Terminal, which is a real
  usability cost. Whether the Mac build is a bundle, a plain binary, or a small wrapper needs
  deciding before that build is added.
- Confirm rather than assume that refbox's existing unsigned Mac builds actually launch on Apple
  Silicon, since an unsigned arm64 binary needs at least an ad-hoc signature to run at all. If
  refbox's bundles work today, whatever makes that true applies here too.

**Requires a CI change** (`.github/workflows/release.yml`), which is shared infrastructure — to be
made deliberately in phase 2, not on this branch.

**DECIDED, Eric, 2026-08-27 — how the operator starts it: open the browser automatically.** On
launch the bridge opens the operator's default browser at its own address. A menu-bar or
system-tray icon was considered and deliberately deferred: it is the more polished answer, and it
is only worth its cost if volunteers actually struggle with the simple version.

The reasoning that settled it: **no interface needs designing, because one already exists.** Tasks
7 and 8 built the operator page — connection status, the refbox picker, the scan, the settings, and
the vMix addresses. The gap was never the interface; it was that a volunteer had to know to open a
browser and type an address, and on a Mac had nothing to double-click at all. So the fix is a
launcher, not a UI.

Rejected: a native window in `iced`. It would duplicate a page a browser already renders correctly,
in a framework this project has repeatedly fought (text that fails to repaint, a canvas that
crashes on the settings page, no table widget), and would leave two interfaces to keep in step.

Kept deliberately: the Windows console window. It is ugly, but it is the off switch a volunteer
intuitively understands — closing it stops the bridge. Hiding it would make the program harder to
quit, not easier.

Pairs with the Mac `.app` bundle, which is four lines of `[package.metadata.bundle]` — refbox's own
is just an identifier, an icon and a minimum OS version.

**Its own branch, not this one** (`.claude/rules/scope.md`: a new concern gets a new branch). Small:
open the browser on startup, plus a flag to suppress it for anyone running the bridge headless.

**Superseded context — the options weighed before that decision:** If this blocks the
machine it was built on, it will block a streaming volunteer's laptop at a venue, probably during
setup and probably with an alarming warning. Phase 1 has no answer today. The realistic options are
code signing (a recurring cost), written instructions for the operator to allow the program, or
making the network sweep something the operator switches on rather than something that happens by
default. Whichever is chosen belongs in phase 2 or in the operator documentation, not in this
branch.

---

## Acceptance

The walkthrough Eric runs, from spec §8. Steps 7 and 8 are the whole design in two actions:

1. Start the bridge; open its status page.
2. It lists refboxes found, each labelled with the game on it. Pick the court.
3. Green indicator, "last heard from: just now", vMix addresses shown.
4. In vMix, add a Data Source pointing at `/scorebug`; bind a title to team names, scores, clock.
5. Score a goal on the refbox — the bug updates.
6. Issue a penalty — it appears with a countdown and the player's name.
7. **Cut the refbox's network mid-half** — within ten to fifteen seconds the graphic stops
   displaying entirely, and the status page goes red. Restore it — the graphic returns with real
   values. At no point does it show a time the refbox did not send.
8. **Stop the clock on the refbox normally** — the graphic **stays on screen**, holding the last
   values, and the status page stays green. This is the one that catches a silence-based
   implementation: a wrong build blanks the graphic here.

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
