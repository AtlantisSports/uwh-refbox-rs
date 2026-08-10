# Game Source Selection — Design

**Status:** approved design, not yet implemented
**Branch:** `feat/refbox/game-source-selection` (off `origin/master` at `dbafd585`)
**Related:** `docs/third-party-integration.md` on `docs/workspace/third-party-data-source`

---

## Why

refbox can already be pointed at a server other than the official UWH Portal, but only by
setting an environment variable and adding a command-line flag before launch. The companion
contract-documentation effort found this to be its single worst gap: the mechanism is invisible
to the operator, undiscoverable without reading source, and when the flag is missing no request
is sent at all — a failure indistinguishable from the server being down.

This design turns that hidden developer switch into a first-class, operator-visible choice.

## Goal

An operator can choose, on screen, whether refbox runs games by hand, from the official Portal,
or from a site of their own — and can see which site is actually in use.

## Scope

**Crate:** `refbox` only. No changes to `uwh-common`, `overlay`, or any other crate.

**Files expected to change:**
- `refbox/src/app/view_builders/configuration.rs` — the settings rows and the source control
- `refbox/src/app/mod.rs` — the source state, the client swap, the CUSTOM fetch trigger
- `refbox/src/config.rs` — persisted source, custom URL, per-source credentials
- `refbox/src/app/view_builders/{game_info.rs, shared_elements.rs, game_info_table.rs}` — these
  currently branch on a boolean and must branch on the new three-way value
- `refbox/translations/*/refbox.ftl` — new labels, all 15 locales

**Explicitly out of scope:**
- The verification stub's token handling.
- Fixes to `docs/third-party-integration.md` (tracked on its own branch).
- Any change to how the official Portal path behaves today, **with one deliberate exception**:
  refbox will stop sending the token-verify call when it holds no token, which affects the Portal
  path as well as the custom one. See "The token indicator" below for why that is in scope here.

---

## What the operator sees

The primary control changes from `USING UWHPORTAL: YES/NO` to `MANUAL GAMES: YES/NO`.

    MANUAL GAMES: YES
        [ MANUAL GAMES: YES ]
        Game number and teams are set by hand. Identical to today's
        "Using UWH Portal: No".

    MANUAL GAMES: NO — two source buttons appear beside it
        [ MANUAL GAMES: NO ][ UWH PORTAL ][ CUSTOM ]

        UWH PORTAL selected — same rows and same flow as today:
        [ EVENT:            Example Open 2026 ]
        [ TOKEN:                          OK  ]
        [ COURT:                           A  ]
        [ CANCEL ][ GAME: 1 ][ APPLY ]

        CUSTOM selected — no event picker; the event is part of the URL:
        [ SITE:  http://scoreboard.local:8099/api/events/1234-A ]
        [ TOKEN:                                            OK ]
        [ COURT:                                             A ]
        [ CANCEL ][ GAME: 1 ][ APPLY ]

Both layouts are four rows, matching today. The two source buttons cost no layout change: row 1
is already a three-cell grid whose second and third cells are blank (`configuration.rs:678`).

In rugby mode the middle button reads `UWR PORTAL`. This needs no new mechanism — the existing
label is already parameterised by tenant (`using-portal = USING { $portal }PORTAL:`,
`en-US/refbox.ftl:107`).

Tapping SITE opens a small page holding the URL field, with Cancel and Apply — the same shape
every other row on this page already uses.

## The custom URL

The operator types one string that carries both the site and the event:

    http://scoreboard.local:8099/api/events/1234-A

refbox splits it into a base URL (`http://scoreboard.local:8099`) and an event ID (`1234-A`).
Both are needed separately, because one call does not live under the event path: the stats push
is `POST /api/admin/events/stats` with the event ID as a query parameter in long form. Splitting
is safe only because the contract fixes the path shape.

**Entry method.** Typed with a physical keyboard attached to the machine. No on-screen keyboard
is built. refbox has no text-entry widget today, so this introduces the first one.

**Encryption follows the scheme of the URL actually in use.** `https://` requires TLS; `http://`
permits plain HTTP. Where the environment override is set it supplies the effective URL, so its
scheme governs — the requirement is always derived from the same URL the SITE row displays.
No launch flag and no extra setting. This matches the sibling CLI tool, which already derives the
requirement from the URL (`schedule-processor/src/main.rs:63`), and removes the silent-failure
mode that motivated this work. No separate "unencrypted" warning is shown: the SITE row displays
the URL, so `http://` is already on screen.

**Validation happens on Apply, not later.** A URL that does not contain the `/api/events/{id}`
shape, or whose event ID would be rejected by the contract's rules, is refused at Apply with a
plain-English message naming what is wrong. Accepting a malformed URL and failing during a game
is far worse for an operator on the pool deck. The ID rules are strict and worth validating
against directly: the prefix is case-sensitive and at least three characters must follow it, and
a violation fails the entire response rather than one field.

Trailing slashes need no handling — the client already trims them (`uwh-common/src/uwhportal/mod.rs:177`).

## Behaviour decisions

1. **A custom site still selects court and game.** It implements the same calls the Portal does,
   so everything below the SITE row is unchanged. CUSTOM changes *where to ask*, not the flow.

2. **The event picker disappears under CUSTOM.** With the event named in the URL, the event-list
   call is not made at all on this path.

3. **Switching to MANUAL = YES preserves the configured source, including which one.** Nothing is
   erased. The custom URL and both tokens persist independently, and the last remote choice is
   remembered separately from the active source, so turning manual off returns the operator to the
   source they were using rather than bouncing them to the portal. An installation that has never
   chosen a source defaults to the portal, so today's behaviour is unchanged.

4. **Each source keeps its own credentials.** The Portal link and the custom site's token are
   stored separately, so moving between them never forces re-linking. This deliberately avoids
   repeating an existing annoyance: switching tenant mode already discards the Portal link and
   makes the operator reconnect.

5. **CUSTOM survives a mode switch; the Portal link still does not.** Changing hockey/rugby mode
   continues to discard the official Portal link exactly as it does today — that behaviour is
   untouched. A custom site is not tenant-scoped in the same way, so it is left alone. The two
   sources therefore behave differently here, which is intentional and must be called out in the
   release notes.

6. **The SITE row always shows the URL actually in use.** The existing environment override
   continues to work for developers, and if it is set, the row shows the effective URL. A typed
   value must never be silently ignored while a different site is really being called — that is
   the exact failure this feature exists to eliminate.

7. **Internally the boolean becomes a three-way value.** `using_uwhportal: bool` is replaced by a
   source enum. The app then cannot represent "using a portal" while pointed nowhere, and the
   compiler locates every screen that must be updated.

## Changing the site at runtime

The portal client is built once, in the app constructor (`mod.rs:1710`, client at `mod.rs:1775`),
so a typed URL needs somewhere to go.

**Apply replaces the client inside its existing lock.** The client is held as
`Option<Arc<Mutex<UwhPortalClient>>>` (`mod.rs:160`), and every request builds its address fresh
from the private `base_url` field. Assigning a newly constructed client through the mutex guard
makes the next request use the new site. `UwhPortalClient::new` is public, and mutation through
this lock is already the established pattern for tokens (`set_token`, `uwh-common/.../mod.rs:187`).
Rebuilding rather than editing a string is also required, because the TLS requirement is set on
the inner HTTP client at construction (`mod.rs:173`).

No app restart. No change to `uwh-common`.

**In-flight requests are already safe.** The code takes the lock, builds the request, releases
the lock, then awaits — see `request_schedule` (`mod.rs:752`). A request already in flight has
its address baked in and completes against the site it was addressed to.

**Two guards, instead of queue-migration logic.** A queued score or stats push carries a game for
the *old* site; replayed against a new site whose schedule has no such event, it would fail and
retry indefinitely. Rather than build queue migration for a rare mid-event reconfiguration:

Both guards apply to any change that would repoint the live client — switching source, and
editing the custom URL:

- Neither can change while a result is pending in the outbound queue. The operator clears it
  first using the existing RETRY ALL and discard actions.
- Neither can change while the clock is running. `clock_running` is already a parameter of the
  settings view builder, so this gate needs no new plumbing.

Both refusals must say why in plain English, not simply grey out.

## How CUSTOM gets its schedule and teams

Today the teams fetch is driven by the event list: `RecvEventList` loops the returned events and
calls `request_teams_list` for each (`mod.rs:4089`). CUSTOM has no event list, so that trigger
disappears and must be replaced explicitly.

**CUSTOM calls the same two helpers directly** for its embedded event ID, on Apply:
- `request_teams_list(event_id)` (`mod.rs:731`)
- `request_schedule(event_id)` (`mod.rs:752`) — which fetches the privileged schedule and the
  referee name map together

No new fetching machinery is required; CUSTOM simply becomes another caller of the functions the
event-list path already loops over.

**One state-shape requirement.** Team data is stored into an entry in the events map, and the
court picker reads the court list from that same entry. CUSTOM must therefore create a single
synthetic entry for the embedded event ID, or teams and court data have nowhere to land and the
court picker stays empty. This is deliberate, not incidental.

## What the token does and does not prove

Four calls are bearer-authenticated: verify token, privileged schedule, push scores, push stats.
The event list, team list and referee list are **not** authenticated at all. So the token protects
writes and the privileged schedule; a custom site's teams and referees are readable by anyone who
can reach it. Where that matters it must be solved at the network level.

**refbox only sends the token. Only the site can enforce it.** This was demonstrated against a
deliberately permissive stub: with no token saved, refbox still issued the verify and privileged
schedule calls with no `Authorization` header at all, the stub answered `200`, and refbox reported
the token as OK and never offered to link. A custom site that accepts anything is therefore wide
open — anyone on the network could push results.

### The token indicator, and why `OK` cannot be trusted

The indicator has three states (`configuration.rs:726-735`): `Some(true)` renders **OK** in green,
`Some(false)` renders **FAILED** in red, and `None` renders **CHECKING...** in grey.

FAILED is the resting state, not an error state. Opening Game Options shows FAILED immediately
when there is no token, and also when there is a token but no event selected yet — the check needs
an event ID to call against (`mod.rs:1612-1626`). Only with both a token and an event does it move
CHECKING... → OK or FAILED.

**What `OK` actually means is "the site answered 200 to our verify call" — not "we are
authenticated."** `check_uwhportal_auth` (`mod.rs:818`) calls `verify_token` unconditionally; it
does not check whether a token exists first. So with no token at all, refbox correctly sets the
indicator to FAILED on event selection (`mod.rs:3594-3605`), then a permissive site's `200`
arrives and `RecvTokenValid` **overwrites it to OK** (`mod.rs:4302`). This was observed live: the
indicator read OK, in green, with no token and no `Authorization` header ever sent.

Two consequences:

1. **Copy decision, open.** For a custom site, OK is misleading. Options: keep `OK`, weaken it
   (`PRESENT`, `ACCEPTED`), or add a short note on the CUSTOM path. To be decided when the UI is
   written; wording, not structure.
2. **Behaviour fix, approved and included here.** When refbox holds no token it does not send the
   verify call at all, and the indicator stays FAILED. No site, however permissive, can then
   report OK without credentials, and `OK` regains a single honest meaning: *we sent a token and
   the site accepted it.*

   Note this deliberately changes the **existing Portal path** as well as the new custom one —
   it is not additive-only. It is included here rather than filed separately because this
   feature's purpose is a source display the operator can trust, and because both changes touch
   the same code, so splitting them would only force one to be rebased over the other. The
   pointless unauthenticated request disappears as a side effect.

## Acceptance criteria

Observable by the human, without reading code:

1. With MANUAL GAMES set to YES, the console behaves exactly as it does today with "Using UWH
   Portal: No".
2. With MANUAL GAMES set to NO and UWH PORTAL chosen, event selection, linking, court and game
   selection behave exactly as they do today.
3. With CUSTOM chosen and a valid URL typed, the court and game can be selected and the game
   loads with both team names — no environment variable and no command-line flag.
4. A URL typed with a bad event ID or a missing path segment is refused at Apply, with a message
   saying what is wrong.
5. Switching UWH PORTAL → CUSTOM → UWH PORTAL does not require re-linking either one.
6. Attempting to change the SITE while the clock runs, or while a result is pending, is refused
   with a reason.
7. Switching hockey/rugby mode leaves a configured custom site intact.
8. The SITE row shows the same URL that requests are actually sent to, including when the
   environment override is set.
9. With no token saved, the token indicator reads FAILED and **stays** FAILED, on both the Portal
   and the custom path — even against a site that answers every request with `200`. Verifiable
   with the existing stub, which is what exposed the old behaviour.

## Risks and open items

- **First text-entry widget in the app.** Focus behaviour on a touchscreen with no keyboard
  attached is untested. Validate strictly on Apply and keep the edit page minimal. Real hardware
  testing on a Pi is required before this ships to a tournament.
- **Fifteen locales.** Every new label needs a real translation in all 15, not a placeholder.
- **Four screens branch on the old boolean.** Converting them is mechanical but touches files
  beyond the settings page; the compiler will find them all.
- **The `TOKEN: OK` wording** is unresolved (above).
- **Client swap under load** is judged safe from the lock-then-build-then-await pattern, but the
  guards exist precisely because that reasoning should not be the only line of defence.

## Evidence

Source references, verified against `dbafd585` on this branch:

| Claim | Location |
|---|---|
| Client held behind a shared lock | `refbox/src/app/mod.rs:160` |
| Client built once, in the constructor | `refbox/src/app/mod.rs:1710`, `:1775` |
| Per-event teams fetch helper | `refbox/src/app/mod.rs:731` |
| Per-event schedule + referees helper | `refbox/src/app/mod.rs:752` |
| Event list drives the teams fetch today | `refbox/src/app/mod.rs:4089` |
| Spacebar gated to the main screen, so text entry is unaffected | `refbox/src/app/mod.rs:4364` |
| Row 1 has two blank cells | `refbox/src/app/view_builders/configuration.rs:678` |
| `base_url` is private and read per call | `uwh-common/src/uwhportal/mod.rs:159` |
| TLS requirement fixed at construction | `uwh-common/src/uwhportal/mod.rs:173` |
| Trailing slash already trimmed | `uwh-common/src/uwhportal/mod.rs:177` |
| Mutation through the lock is established | `uwh-common/src/uwhportal/mod.rs:187` |
| refbox derives TLS from the launch flag today | `refbox/src/main.rs:665` |
| Sibling tool derives TLS from the URL scheme | `schedule-processor/src/main.rs:63` |
| Tenant-parameterised label already exists | `refbox/translations/en-US/refbox.ftl:107` |
