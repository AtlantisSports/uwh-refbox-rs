# Alternate overlay delivery: a bridge for vMix and third parties

**Date:** 2026-08-26
**Branch:** `docs/overlay/alternate-delivery-options`
**Status:** Design agreed in conversation; awaiting review of this document before an
implementation plan is written.

---

## 1. Goal

Let a stream produced with vMix (or comparable software) on a PC on the same network as the
refbox show live game graphics, **without** the Raspberry Pi plus capture-card arrangement the
current overlay requires — and let third parties consume the same live game data in their own
programs and websites.

This builds on the third-party integration contract merged in PR #2493. That document describes
the calls the refbox **makes to a website**. This work is the mirror image: the feed the refbox
**serves to the local network**, which has never been documented.

---

## 2. Scope boundary

**In scope**

- A new program (the "bridge") that runs on the streaming PC.
- A written contract document for the refbox's existing network feed.
- Guidance for building the vMix titles that consume it.

**Explicitly not in scope**

- **Any change to the refbox application.** Nothing in `refbox/`, `uwh-common/`, or the shared
  data format. See §4.2 — this is a deliberate design outcome, not an omission.
- **Any change to the existing overlay program.** It keeps working exactly as it does today.
- Anything touching the LED panel, the wireless remote, or the binary feed.
- Retiring the current Pi + capture-card setup. This adds a second route; it replaces nothing.
- How the **web refbox** publishes its game state. That is a real and separate gap (§9.1).

---

## 3. What already exists

Every claim here was verified against the source. Citations in §11.

**The refbox is already a live data server on the local network.** It listens on two TCP ports:
a binary feed for the LED panel (default 8001) and **a plain-JSON feed for the overlay
(default 8000)**. Anything on the same network can connect to the JSON port today, with no
password and no Portal involvement.

**Each update is one JSON object followed by a newline.** A consumer should read to the newline.
Our own overlay does not — it reads up to 1024 bytes and parses whatever arrived (§10.1).

**A new connection is answered immediately with the current state.** A consumer that starts
mid-game is correct from its first second.

**While the clock is running, an update is produced once per second, on the second.** When the
clock is stopped, the component that produces updates sleeps until the clock starts again. This
asymmetry is the foundation of §5.4.

**The feed is strictly one-way.** The code serving each connected client is built against a
write-only interface and never reads. Nothing a consumer sends can reach a game.

**What the feed carries:** period, seconds remaining, timeout kind and countdown, both scores,
penalties (cap number, time remaining, infraction), warnings and fouls (cap number, infraction,
including a "neither team" bucket), the most recent goal (colour and cap number), this and the
next game number, the event ID, and the length of the next period.

**What the feed does not carry:** team names, player names, rosters, flags, logos, court, start
time, and **which team is on which side of the pool** — that last one exists but is sent only to
the LED panel. The "hide time" setting likewise affects only the LED panel; the feed always
carries the true clock.

**How the current overlay fills the gaps.** It fetches team names, rosters, player photos, flags,
event logos, referees and the schedule from the Portal, **with no credentials of any kind**. Its
entire configuration is three values: the refbox's address, its port, and the Portal address.

**The current overlay is a renderer.** It opens a fixed 3840x1080 window and draws each graphic
twice side by side — the colour version and a black-and-white cut-out — which is the "fill and
key" pair broadcast hardware expects, delivered by pointing a capture card at a screen.

---

## 4. Decisions

### 4.1 vMix draws the graphics; we supply data

We stop rendering pictures for this route and feed vMix the numbers and names. Its own title
system draws the scorebug, with its normal transparency, animation and design tools.

*Why:* one surface serves us and third parties identically, and anyone can restyle the look
without touching our code. The existing overlay's artwork does not carry over; its **layout
decisions** carry over as vMix title designs (§7, phase 2).

### 4.2 The refbox is not modified

The bridge works out everything it needs from the feed as it exists today.

*Why:* it works with **every refbox already in the field, unchanged** — no rollout, no version
matching, and no risk to the LED panel or the existing overlay. A third party can build against
boxes as they actually are rather than as they will be after an update. This reverses an earlier
draft of this design, which proposed adding a clock-running flag and a heartbeat to the feed;
§5.4 explains why those turned out to be unnecessary.

### 4.3 A separate program on the streaming PC, not a web server on the Pi

*Why:* a poller hitting the Pi ten times a second is far more network traffic than the single
push connection it serves today, and the Pi's link is the weak one. More importantly, a bridge on
the streaming PC **holds last-known state on a wired connection that does not drop**, so a Wi-Fi
hiccup at the Pi never reaches air.

### 4.4 The bridge is a downloadable program, not a page in the Portal

*Why:* a normal web page cannot open the kind of network connection the refbox serves — browsers
allow only HTTP and WebSockets, and a page loaded over a secure connection may not open an
insecure one to a local device. A browser tab also has no address for vMix to poll, and a page
served from uwhportal.com needs internet at a venue where internet is the unreliable part.

### 4.5 The feed carries cap numbers; the receiver resolves names

*Why:* this is what the current overlay does. The refbox says "black #7 scored"; the overlay
turns that into `#7 SMITH` using the roster it fetched itself. A third party running their own
site already has their own roster.

### 4.6 On losing contact, the picture stays correct and only the operator is warned

*Why:* a frozen clock is wrong information that happens to sit still, and it is wrong by the full
length of the outage. The alarm belongs where the crew sees it, not on air.

---

## 5. The bridge

### 5.1 What it is

A small program that runs on the streaming PC. It connects to the refbox exactly as the overlay
does, holds the current picture of the game, and serves that picture at a local web address that
vMix polls. It has no window of its own; its status page is a web page (§5.6).

Primary target is Windows, because vMix is Windows-only. The workspace already cross-compiles
`schedule-processor` to Windows, so the precedent exists.

### 5.2 What it serves

A local web server, **default port 8099**. (Not 8088 — that is vMix's own web controller.)

Everything is served as a **table**, because that is what title systems consume: a title binds to
a row and pulls columns into text fields.

| Address | Shape | Contents |
|---|---|---|
| `/scorebug` | one row | Both team names, both scores, clock, period, timeout kind and countdown |
| `/penalties` | one row per active penalty | Team, cap number, player name, time remaining or `TD` |
| `/fouls`, `/warnings` | one row each | Team, cap number, player name, infraction |
| `/nextgame` | one row | Next game's teams, court, scheduled start |
| `/status` | operator page | Connection state, last contact, settings (§5.6) |

**Every time value is served twice** — once as display-ready text (`"3:47"`, `"1:42"`, `"TD"`) and
once as a plain number (`227`). Title systems cannot do arithmetic; other software wants the
number.

**The clock is recalculated on every request** from the bridge's own count, so what vMix reads is
current to the moment rather than current as of the refbox's last message.

**Tables are a fixed length, blank-padded.** vMix titles bind to an explicit row number, so a
table whose length varies leaves a title bound to row 3 reading stale or missing data whenever
fewer than three penalties are active. `/penalties`, `/fouls` and `/warnings` are therefore always
served at a fixed row count with empty rows padding the remainder. The ceiling must be checked
against what a game can actually produce before it is fixed.

**Column names are part of the contract.** vMix matches columns to title fields by name, so
renaming one silently breaks every title built against it. Names are chosen once and then treated
as published.

**Two settings the feed cannot supply:** which team is on which side of the pool, and the court,
if it cannot be derived. Set once per session, not per game.

Exact shapes, the vMix setup steps and the gotchas behind these two rules are in the companion
document `2026-08-26-vmix-integration-steps.md`.

### 5.3 Setup and discovery

Configuration is effectively one value — the refbox's address — because everything else defaults
and **no login is required anywhere** (§3).

**Typing that address is not the normal path.** The refbox does not display its own network
address anywhere in the application, so finding it means leaving the refbox screen for the
machine's operating-system settings. Instead the bridge checks the local network and, for
anything that answers, **confirms it is really a refbox** — because a refbox sends the current
game state the instant anything connects. The operator then picks by what is on each box:

```
Found 2 refboxes:
  192.168.1.50   Game 14 - Second Half - 3:47 - 2-1
  192.168.1.51   Game 15 - Between Games
```

Manual entry remains available (a first network check may raise a Windows firewall prompt, and
some venue networks block it), and the last address used is remembered.

**The event and game identify themselves.** Every update carries the event ID and game number, so
nobody tells the bridge which tournament it is at. When the game changes, it fetches the new teams.

### 5.4 When contact with the refbox is lost

The bridge decides for itself whether the clock was running, from whether it was being fed:

- **Updates were arriving, then stopped** -> the clock was running -> **keep counting.**
- **Updates were not arriving** -> the clock was stopped -> **hold everything.**

Both are the correct display behaviour, and both are derived without help from the refbox. It is
self-correcting: the instant contact returns, real values overwrite the estimate.

The residual gap is a **long legitimate stoppage** — half time, an injury — during which the link
dies. Silence is expected there, so the bridge has no reason to suspect anything; the picture is
still correct, but the operator is not warned promptly. Closed on our side by keeping the
connection actively checked, which requires nothing from the refbox. Implementation detail for
the plan.

Sudden death counts **up**, not down; the period is in the feed, so the local count handles it.

### 5.5 When the Portal is unreachable

The bridge retries on a timer, caches everything it successfully fetched, and keeps serving the
last good version. If it has never fetched a roster it shows cap numbers without names. **Game
information from the refbox is never affected by the Portal being down**, because it does not
come from there.

This is deliberately the opposite of the current overlay's behaviour (§10.2).

### 5.6 The status page

Served by the bridge at its own address, available the moment it starts and before any refbox is
configured, so there is no chicken-and-egg. It shows:

- A large green/red indicator, and how long since the refbox last spoke.
- The current event, game and period.
- The discovery list and the manual address field.
- The two operator settings (§5.2).
- The addresses to paste into vMix.

It can be left open on a second monitor, or checked from a phone on the same network.

---

## 6. The third-party contract document

A companion to `docs/third-party-integration.md`, living beside it. Covers:

- **Where the feed is and how to read it** — port, all interfaces, one JSON object per update
  terminated by a newline, read to the newline.
- **When it speaks** — replay on connect; on every change; once per second on the second while
  the clock runs — and how to use that last fact to detect a lost refbox without our help.
- **Every field and its meaning**, including that penalties, fouls and warnings carry cap numbers
  and that resolving names is the consumer's job.
- **What is deliberately absent** — side of pool, team names, venue information — stated plainly
  so nobody wastes a day looking.
- **Two security statements.** It is one-way; nothing sent to it can affect a game (provable,
  §3). And it has no password: anyone on the network can read it. Acceptable because the content
  is what a scoreboard already shows in public, but it belongs on a tournament network, not the
  open internet.
- **What we commit to not breaking** — fields may be added; existing fields will not change
  meaning without notice; consumers should ignore fields they do not recognise.
- **The limits, stated up front** — what has been verified against a real refbox and what has not.

**Written last, deliberately.** The merged third-party document was factually wrong in five
separate places when written ahead of a working implementation, and only building something
against it found them. A contract written after a real consumer exists describes what actually
happens.

---

## 7. Plan of record

| Phase | What | Who |
|---|---|---|
| **0a** | Establish vMix's documented JSON requirements; write the integration steps as a mock walkthrough | Claude - **DONE**, see `2026-08-26-vmix-integration-steps.md` |
| **0b** | Throwaway "peek" tool: connect to a refbox, print exactly what arrives | Claude |
| **1** | The bridge | Claude |
| **2** | vMix titles recreating what the overlay shows today | Eric / titles builder |
| **3** | The third-party contract document | Claude |

**No phase gates on running vMix.** An earlier version of this plan made a live vMix test the gate
on everything. That over-weighted it: **the output shape is the cheapest part of the bridge to
change.** Connecting, holding state, judging whether the clock was running, keeping count, caching
rosters and surviving a Portal outage are all shape-independent; the served format is a thin layer
over them. The live run therefore moves to **the end of phase 1**, where it tests the real bridge
rather than a static file, and proves the whole chain instead of one assumption.

**What 0a establishes without launching anything.** Inspection of the installation has already
confirmed the two properties the design depends on: a dedicated JSON data source that accepts **a
URL as well as a file**, and a refresh interval expressed **in milliseconds**, so a sub-second
clock is supported by design. What remains for 0a is the required JSON *structure* — how a data
source turns a document into rows and columns, and the practical minimum refresh — which is
documented rather than discovered. 0a's second half is the mock walkthrough: the vMix setup steps
written out before vMix is opened, which becomes the phase-2 starting point and, later, the setup
guide shipped to operators and third parties.

**0b** is worth doing because nobody has looked at what actually crosses that wire -- only at the
code that writes it and the code that reads it. It either confirms the field list in §3 or
catches a discrepancy now rather than mid-build.

**Eric's original ordering** put a cross-PC data test first. That is skipped as a phase because it
is already proven in production: the overlay runs on the stream computer and connects to the
refbox across the network at every major tournament. The cross-PC run still happens, as the
bridge's first real-world test in phase 1, where it proves something new.

---

## 8. Acceptance criteria

Walked by Eric, on real hardware:

1. Start the bridge on the streaming PC; open its status page.
2. It lists refboxes found, each labelled with the game on it. Pick the court.
3. Indicator green, "last heard from: just now", vMix addresses shown.
4. In vMix, add a Data Source pointing at `/scorebug`; bind a title to team names, scores, clock.
5. Score a goal on the refbox — the bug updates.
6. Issue a penalty — it appears with a countdown and the player's name, matching the overlay.
7. **Unplug the refbox's network mid-half** — the clock keeps counting on screen; the status page
   goes red with time since last contact. Reconnect — the clock corrects silently.
8. **Stop the clock on the refbox normally** — the clock holds; the status page stays green.

Steps 7 and 8 are the entire design in two actions.

Automated tests from day one for: the clock continuation, the "were updates arriving" judgement,
roster caching and the fall back to bare cap numbers, and the served table shapes. The overlay
crate ran for years with no tests at all; its first two arrived in August 2026. The bridge does
not repeat that.

Before a tournament: it runs on the real streaming PC; it survives a genuinely weak link rather
than a simulated one; and a full game runs start to finish without attention.

---

## 9. Risks and open questions

### 9.1 The web refbox cannot be observed at all

The web refbox holds its game state **inside the browser** (`gameState.ts`,
`gameStatePersistence.ts` in the portal repo); nothing on the portal's server side holds or
broadcasts a live game. **So nothing outside that browser tab can see a game run on the web
refbox** — no overlay, no LED panel, no stream, no third party.

That is a real gap and arguably the more important long-term question. It is deliberately not
answered here. The mitigation is architectural: **the bridge's output shape is the standard, and
more than one thing may fill it.** Today it is filled by reading the Rust refbox over the local
network; a web-refbox game could later fill the same shape a different way, and stream setups
built against it would not care which refbox is running.

### 9.2 vMix behaviour is confirmed by inspection and documentation, not yet by a live run

The remaining unknowns are the title-binding step and the practical minimum refresh interval,
both settled at the end of phase 1 (§7). Accepted rather than mitigated, because the served shape
is the cheapest part of the bridge to change if it turns out wrong.

### 9.3 Network discovery may be blocked

Venue networks and Windows firewall may interfere with §5.3. Mitigated by keeping manual entry
and remembering the last address.

### 9.4 The dropout judgement is inference, not fact

§5.4 reasons from observed behaviour rather than a stated flag. It will be right in ordinary
cases and self-corrects on reconnection, but corner cases exist — for example, the referee
stopping the clock at the exact moment the link dies. If field experience shows it is
insufficient, adding an explicit flag to the refbox remains available as a later improvement,
taken with evidence.

---

## 10. Defects found during design — out of scope, each needs its own branch

### 10.1 The overlay can silently discard a large update

The feed terminates each update with a newline, but the overlay reads up to 1024 bytes and parses
whatever arrived (`overlay/src/network.rs:492-505`). A game with many penalties, fouls and
warnings could exceed that; the update is then discarded as corrupt with only a log warning.

### 10.2 The overlay loses a game's team data permanently if the Portal is briefly unreachable

A failed team-information request is treated as fatal for that lookup
(`overlay/src/network.rs:100-105`, and the same for event logos at `179-184`). It does not bring
the overlay down, but that fetch dies and never retries, so **that game's team names and flags
never appear** and nothing on screen explains why. It recovers only at the next game change.

### 10.3 The refbox never displays its own network address

There is no such screen and no such wording in the application. Setting up the overlay, the LED
panel or the bridge therefore requires leaving the refbox for the machine's operating-system
settings. Eric has confirmed the current workflow is acceptable and asked for no change; recorded
here only as an observation.

---

## 11. Verified facts and citations

| Claim | Source |
|---|---|
| JSON feed default port 8000, binary 8001 | `refbox/src/main.rs:138-144` |
| Both listeners bound on IPv4 and IPv6 | `refbox/src/app/update_sender.rs:721-742` |
| Payload is the game state as JSON plus a newline | `refbox/src/app/update_sender.rs:494-496` |
| Current state replayed to a new connection | `refbox/src/app/update_sender.rs:606-630` |
| Client connections are write-only; never read | `refbox/src/app/update_sender.rs:209,216` |
| Side-of-pool goes only to the binary panel frame | `refbox/src/app/update_sender.rs:536-545` |
| "Hide time" applied after JSON encoding | `refbox/src/app/update_sender.rs:494-527` |
| Updates scheduled on the next whole second | `refbox/src/tournament_manager/mod.rs:2372-2402` |
| Updater sleeps while the clock is stopped | `refbox/src/app/mod.rs:6890-6897` |
| Feed field list | `uwh-common/src/game_snapshot.rs:42-57` |
| Penalties and infractions carry cap numbers, not names | `uwh-common/src/game_snapshot.rs:115-125` |
| Sudden death counts up | `uwh-common/src/game_snapshot.rs:203` |
| Overlay configuration is three values | `overlay/src/main.rs:34-48` |
| Overlay sends no credentials anywhere | searched `overlay/src/` — no token, bearer or key |
| Overlay reads 1024 bytes and parses the buffer | `overlay/src/network.rs:492-505` |
| Overlay resolves names and draws `#N NAME` | `overlay/src/flag.rs:66-77,344,478` |
| Overlay treats a failed team fetch as fatal for that fetch | `overlay/src/network.rs:100-105,179-184` |
| Overlay window is 3840x1080 (fill plus key) | `overlay/src/main.rs:413-418` |
| Refbox has no screen showing its own address | searched `refbox/src/` and `refbox/translations/en-US/refbox.ftl` |
| Web refbox state lives in the browser | uwh-portal `js/@underwater-web/lib/refbox/gameState.ts`, `gameStatePersistence.ts` |
| vMix ships a dedicated JSON data source | `vMix/datasources/JSONDataSource.dll` |
| vMix data sources take a URL or a file, refresh in milliseconds | `vMix/DataSourceAPI.dll` — `get_URL`, `IsURLFilename`, `get/set_TimerMilliseconds` |
| vMix's own web controller uses port 8088 | `vMix/vMix64.exe` |
