# Third-party data source — design

**Date:** 2026-08-07
**Status:** Design agreed. Nothing built yet.
**Crates in scope:** `refbox` (the bulk), `schedule-processor` (one menu entry),
`overlay` (documentation only), `uwh-common` (address plumbing).

---

## The problem

Today the refbox can talk to the UWH Portal, or to nothing at all. A third party
who wants to use our timing software but not the Portal — a national federation,
a league running its own scores site, a club with its own system — has no
supported way in.

The goal of this design is to give them one, and to do it without taking on an
open-ended maintenance obligation.

## What we found before designing anything

The gap is much narrower than it first appears:

- The **overlay** already reads its Portal address from its own config file
  (`overlay/src/main.rs:37`). It will point at any address today, with no change.
- **refbox** and **schedule-processor** already honour a `UWH_PORTAL_URL_OVERRIDE`
  environment variable that does the same thing, used for testing against the
  development Portal.
- **schedule-processor already has the picker** this design adds to refbox — it
  asks which site (Production / Development / Local) crossed with which sport
  (`schedule-processor/src/main.rs:124`).
- The **UWR Portal is already a "third-party site"** as far as the code is
  concerned: identical code, identical calls, different address, chosen by sport
  mode. It is proof the pattern works.
- All the reliability machinery — the retry queue, the health indicator, the
  stuck-result handling, surviving being offline — lives in `refbox/src/portal_manager/`
  and does not know or care who is on the other end.

So nothing needs inventing. What is missing is an operator-facing *choice*, an
address stored somewhere permanent instead of an environment variable, and a
written description of what the other end has to answer.

## Decisions taken

| Decision | Choice |
|---|---|
| Driver | Scoping effort before committing. No partner waiting. |
| Data scope | Everything, full fidelity — schedule, teams, rosters, referees, coin flips in; scores and player-attributed stats out. |
| Programs covered | All three: refbox, overlay, schedule-processor. |
| Commitment level | Best effort. Documented, no stability promise. |
| Approach | **A — Portal-compatible.** The third party matches our existing API; we add the picker and store the address properly. |
| Text entry | Included. A physical keyboard, a typed-entry page, and a TEST button. |
| Logins | **One shared login slot**, as today. Switching sites clears it. |
| Third-party login | **Typed key for custom sites.** Built-in Portals keep the six-digit link flow unchanged. |
| Sequencing | **Contract document first**, then refbox, then the two small tools. |

### Approaches considered and rejected

**B — a second, simpler dialect.** Design a deliberately small interface (6–8
calls) that refbox also speaks, far easier for a third party to implement. Rejected
because it is the largest and most permanent cost on our side — a second
implementation of every call, translation into our internal types, and two code
paths that must both keep working forever. It only pays for itself with several
third parties or a real stability promise, and we have neither.

**C — translator in the middle.** The third party runs a shim speaking Portal
language on one side and their own system on the other. Rejected as a *design*
because it is what we already have if we build nothing: no operator-facing choice,
configured invisibly through environment variables. The technical bargain is
identical to A; A simply makes it visible and operator-friendly.

---

## Section 1 — What the operator sees

refbox already has a picker page of exactly the right shape: the one used to choose
the Event, Court and Game (`refbox/src/app/view_builders/list_selector.rs`). A
scrolling list of four items with up/down arrows and a Cancel button. The data
source picker becomes a fourth thing that page can list, inheriting its look,
scrolling and behaviour.

**Today** (`refbox/src/app/view_builders/configuration.rs:650`):

```
┌───────────────────────────────────────────────┐
│                    12:34                      │
├───────────────┬──────────────┬────────────────┤
│ USING UWH     │              │                │
│ PORTAL        │              │                │
│      Yes      │              │                │
├───────────────┴──────────────┴────────────────┤
│ EVENT                                         │
│      San Diego Beach Bash                     │
├───────────────────────────────────────────────┤
│ TOKEN                                         │
│      Valid                                    │
├───────────────────────────────────────────────┤
│ COURT                                         │
│      Court 1                                  │
├──────────┬───────────────┬────────────────────┤
│  CANCEL  │     GAME      │       APPLY        │
└──────────┴───────────────┴────────────────────┘
```

**Proposed** — the Yes/No button becomes a value button that opens the picker.
Everything below it is unchanged:

```
┌───────────────────────────────────────────────┐
│                    12:34                      │
├───────────────┬──────────────┬────────────────┤
│ DATA SOURCE   │              │                │
│  UWH Portal   │              │                │
├───────────────┴──────────────┴────────────────┤
│ EVENT                                         │
│      San Diego Beach Bash                     │
├───────────────────────────────────────────────┤
│ TOKEN                                         │
│      Valid                                    │
├───────────────────────────────────────────────┤
│ COURT                                         │
│      Court 1                                  │
├──────────┬───────────────┬────────────────────┤
│  CANCEL  │     GAME      │       APPLY        │
└──────────┴───────────────┴────────────────────┘
```

**The picker** — same furniture as the existing Event/Court/Game picker:

```
┌───────────────────────────────────────────────┐
│                    12:34                      │
├────────────────────────────────────┬──────────┤
│          SELECT DATA SOURCE        │          │
│  ┌──────────────────────────────┐  │    ▲     │
│  │ None — standalone            │  │          │
│  ├──────────────────────────────┤  │          │
│  │ UWH Portal                   │  │    ▼     │
│  ├──────────────────────────────┤  │          │
│  │ Pacific League Scores        │  │          │
│  ├──────────────────────────────┤  │  CANCEL  │
│  │ + Add a site…                │  │          │
│  └──────────────────────────────┘  │          │
└────────────────────────────────────┴──────────┘
```

"None — standalone" is what today's **No** means: refbox works entirely on its own
and the operator types in the teams. In rugby mode the second entry reads **UWR
Portal**, exactly as the label does today.

**"+ Add a site…"** opens the one genuinely new page:

```
┌───────────────────────────────────────────────┐
│                    12:34                      │
├───────────────────────────────────────────────┤
│  NAME                                         │
│  ┌─────────────────────────────────────────┐  │
│  │ Pacific League Scores▌                  │  │
│  └─────────────────────────────────────────┘  │
│  ADDRESS                                      │
│  ┌─────────────────────────────────────────┐  │
│  │ https://scores.pacificleague.org        │  │
│  └─────────────────────────────────────────┘  │
│  KEY                                          │
│  ┌─────────────────────────────────────────┐  │
│  │ ••••••••••••••••                        │  │
│  └─────────────────────────────────────────┘  │
│                                               │
│  ✓  Site answered correctly                   │
├──────────┬───────────────┬────────────────────┤
│  CANCEL  │     TEST      │        SAVE        │
└──────────┴───────────────┴────────────────────┘
```

Three deliberate choices:

- **SAVE stays greyed out until TEST succeeds.** This is the existing "Apply is
  disabled until the values are valid" pattern, and it turns a silent mistyped-address
  failure into a clear message before the tournament rather than during it.
- **While any box has the cursor in it, the keyboard stops controlling the game.**
  refbox treats the spacebar as a game control; without this, typing an address
  would start and stop the clock. A gating mechanism for exactly this already exists.
- **The key field is only shown for custom sites.** Built-in Portals use the
  existing six-digit link flow and have no key to type.

Once a site is saved it is a permanent entry in the list, so it is typed once and
picked thereafter — including by anyone with no keyboard to hand.

### Text entry: why this is affordable

Checked, not assumed:

- refbox already listens to physical key presses — `iced::keyboard::on_key_press`
  and `on_key_release` subscriptions in `refbox/src/app/mod.rs`. That is how the
  spacebar shortcut works. A USB keyboard on the Pi reaches the app today.
- The text-box widget is built into iced 0.13 and is not feature-gated
  (`iced_widget-0.13.4/src/lib.rs:33`). Nothing to add or upgrade.
- refbox has **no text entry anywhere today** — every input is a numeric keypad or
  a value button, and the Portal login page only *displays* a number. So this page
  has no sibling to mirror, and designing it is the bulk of the effort rather than
  the widget itself.

---

## Section 2 — What gets stored, and where

refbox already splits this in a way worth preserving:

- **`config.toml`** holds the Portal token (`refbox/src/config.rs:63`) — one token,
  no address. The address comes from the sport mode or an environment variable.
- **`portal_link.json`**, next to the retry queue, remembers the live link: which
  event, which court. It is versioned, survives restarts, and recovers gracefully
  from a damaged file (`refbox/src/portal_manager/link_session.rs`).
- The current on/off flag is in neither. It resets to off every launch and is
  re-established from the remembered link.

The design follows the same split:

**Into `config.toml`** — a list of sites, each with a name and an address. UWH
Portal and UWR Portal are always present as built-ins with their addresses baked
in, so they cannot be deleted or mistyped. Sites added through the typed-entry page
are appended.

The single existing token field stays as it is, and **a typed key is stored in that
same field** — it is the saved login for whichever site is selected, whether it
arrived via the six-digit link flow or was typed in directly. TEST uses whatever key
is in the box at the time, so a site that requires a key is verified with it rather
than being tested anonymously and failing later.

**Into `portal_link.json`** — which site is currently selected, beside the event and
court it belongs with. "Which site, which event, which court" is one coherent
thought, and that file already knows how to survive a restart.

**Unchanged** — `UWH_PORTAL_URL_OVERRIDE` keeps working and keeps applying only to
the built-in Portal entry, so development testing is untouched.

**One shared login slot.** Switching sites clears the saved login and the new site
must be linked or keyed. This matches today's single-token behaviour and has a
safety property worth naming: **a Portal token can never be sent to a third party's
address**, because switching wipes it first.

**Existing setups keep working.** refbox already has a config-upgrade mechanism
(`refbox/src/config.rs:68`). Anyone who never opens the new picker sees no change.

---

## Section 3 — What changes in each program

**refbox — about 90% of the work.**

- The site list in `config.toml`; the selected site in `portal_link.json`.
- The picker: a fourth thing the existing list-selector page can list. Small.
- The typed-entry page (NAME / ADDRESS / KEY / TEST / SAVE). The only genuinely
  new UI in the design, and the one carrying the spacebar-conflict risk.
- The address handed to the connection code instead of being derived from sport
  mode and environment variables.
- **Untouched:** all 18 calls, the retry queue, the health indicator, stuck-result
  handling, the queue file.

**overlay — no code at all.** Its address is already a setting in its own config
file. The work is documentation: overlay makes several calls with its own code
rather than through the shared connection code, including team images, so a third
party wanting stream graphics must answer those too.

**schedule-processor — one menu entry.** Add "Other…" alongside Production /
Development / Local and let the admin type an address. It is a command-line tool
run on a computer, so typing is free and none of the touchscreen problem applies.

---

## Section 4 — The contract document (deliverable one)

All 18 calls, grouped by which program makes them, with exact addresses, what
refbox sends, what it expects back, and every awkward detail spelled out. It
describes what already exists, so it costs care rather than invention.

### The 17 endpoints / 18 operations

Seventeen live in `uwh-common/src/uwhportal/mod.rs`, shared by refbox and
schedule-processor; one more is overlay-only, made by the overlay's own code. Coin
flips share a path between read and write, hence 17 paths for 18 operations.

| Operation | Path |
|---|---|
| Link a refbox | `POST /api/events/{id}/access-keys/ref-box` |
| Log in with email and password | `POST /api/authentication` |
| Verify a token | `GET /api/events/{id}/access-keys/verify` |
| Push game stats | `POST /api/admin/events/stats` |
| Push game scores | `POST /api/events/{id}/schedule/games/{n}/scores` |
| Schedule (privileged) | `GET /api/events/{id}/schedule/privileged` |
| Schedule (public) | `GET /api/events/{id}/schedule` |
| Event list | `GET /api/events` |
| Event teams | `GET /api/events/{id}/teams` |
| Team roster | `GET /api/admin/get-event-team` |
| Referees | `GET /api/events/{id}/referees` |
| Participants | `GET /api/events/{id}/participants` |
| Game referees | `GET /api/admin/events/game-referees` |
| Coin flips (read) | `GET /api/events/{slug}/schedule/coin-flips` |
| Coin flips (write) | `POST /api/events/{slug}/schedule/coin-flips` |
| Upload a schedule | `POST /api/events/{slug}/schedule` |
| Map teams | `POST /api/events/{slug}/schedule/map-teams` |
| Overlay images | `GET /api/admin/events/{id}/overlay-attachments` |

### The awkward parts, named honestly

These are the reasons a third party might give up, and they belong in the document
rather than being discovered later:

- **Identifiers carry a database's fingerprints.** Team IDs look like
  `teams/10753-A` — the internal format of the database the Portal happens to use.
  A third party must mint IDs in that shape.
- **Event IDs appear in two forms** depending on the call, a full form and a short
  form, and which one a call wants is not guessable — only documented.
- **The stats feed uses tagged records.** Each goal, penalty and foul is labelled
  with a `$type` field and its own timestamp format
  (`refbox/src/tournament_manager/game_stats.rs`). It is the deepest, most
  Portal-shaped part of the surface.
- **Some failures are read by their text.** A failed login is matched against the
  words `NoPendingLink` and `InvalidCode`, and refbox behaves differently for each.
  Wrong spelling, wrong message to the operator.

### Logins

Built-in Portals keep today's flow, unchanged: refbox shows a random six-digit
number; an admin enters it on the Portal website; the website shows a code; the
operator types it into refbox; refbox trades the pair for a token
(`uwh-common/src/uwhportal/mod.rs:206`).

**Custom sites use a typed key instead.** That flow is not an API — it is an
interactive website with an approval screen, which is a different order of work
from answering 18 calls and the single most likely reason a third party would walk
away. Removing it costs us one text field on a page we are already building.

---

## Section 5 — When it goes wrong

Most of this is already solved and does not change: the retry queue survives being
offline, the health indicator shows trouble, stuck results get a RETRY ALL.

Three failure modes are new:

**A site that answers but answers wrongly.** During a third party's development
this is the *normal* case. TEST must do more than ping: it fetches the event list
and checks the reply parses, and distinguishes "could not reach that address" from
"reached it, but the reply was not in the expected form". That distinction is the
difference between a useful button and a decorative one.

**Switching sites with results still queued.** A queued result records the event and
game number but *not which site it belongs to* (`refbox/src/portal_manager/queue.rs:44`).
Switching would try to deliver old results to the new site — and under a shared
login the old site's credentials are gone anyway, so they could never be delivered.
**Block switching while anything is unsent**, with a message saying how many results
are waiting. Tagging each queued result with its site was considered and rejected:
more code for a case that cannot work under a shared login.

**Plain, unencrypted addresses.** A small league's server may have no security
certificate. Allow it, and have TEST say plainly that the connection is unencrypted
and the key travels in the clear. Blocking it outright would rule out exactly the
private local servers this feature is for.

---

## Section 6 — How we would know it works

**The acceptance test for the document is a fake site.** If someone can build a
working stub from the document alone and a full game runs through it, the document
is good. If they cannot, the effort belongs in the document, not the picker. This
is the gate on deliverable one and the reason it comes first.

Beyond that:

- The development Portal already exists at a different address. Once the picker is
  built, adding it as a typed site and running a full game through it exercises the
  entire path against a real server.
- Tests for the config upgrade and for the picker's contents. `list_selector.rs`
  already has tests to mirror.
- One hardware check that matters more than it sounds: **type an address while a
  game is running and confirm the clock does not move.**

---

## Sequencing

1. **The contract document.** Cheapest item, describes what already exists, and it
   is the only thing that answers "would a third party actually do this?" If the
   write-up turns out to be forbidding — and the ID formats and stats structure
   might well make it so — that is worth knowing before building a picker for a
   door nobody will walk through. It is also the only deliverable a third party can
   act on.
2. **refbox** — picker, typed-entry page, config storage, address plumbing.
3. **schedule-processor and overlay** — one menu entry and a documentation section.

Each is a separate branch. Step 1 gates the rest: review the document before
committing to step 2.

## Explicitly not doing

- **No new simpler interface.** Third parties match the existing API. Approach B
  stays available later as just another entry in the picker if the document proves
  too forbidding.
- **No stability promise.** Best effort. A third party tracks our releases at their
  own risk, and nothing in our release process changes to accommodate them.
- **No change to the 18 calls themselves**, to the retry queue, to the health
  indicator, or to stuck-result handling.
- **No change to how the Portal itself is used.** Existing setups behave exactly as
  they do now.
- **No on-screen software keyboard.** Typed entry assumes a physical keyboard,
  used once at setup.

## Risks

- **The typed-entry page has no sibling to mirror**, which is against the grain of
  how this codebase is normally extended. Expect the design of that page to take
  longer than its implementation.
- **The spacebar conflict is the most likely source of a serious bug** — a text box
  that leaks key presses into the game controls would be discovered mid-game.
- **The contract document ages silently.** Nothing in CI checks it against the code,
  so it will drift. Under a best-effort commitment that is acceptable, but it should
  carry a visible "accurate as of version X" marker.
