# Degraded portal startup: stop blaming the operator's login

**Date:** 2026-08-13
**Crate:** `refbox` only
**Origin:** finding 3 of `docs/audit-archive/2026-08-11-ai-slop-inventory.md` (reachable only via
`git show 87be104b:docs/audit-archive/2026-08-11-ai-slop-inventory.md`)

---

## The problem in one sentence

When the portal subsystem fails to start because of a fault in the machine, the refbox tells the
operator that their access token expired and pushes them into a re-login that cannot even send a
request.

## What actually happens today

The refbox has a "degraded mode" for when the portal subsystem cannot start. It is deliberate and
correct in spirit: the game clock and scoring keep working, which is what matters at the pool, and
the portal dot turns red so the operator knows something is wrong.

The defect is in *what the red dot says*. Degraded startup sets the flag that means "the portal
rejected our saved token" (`portal_manager/mod.rs:613`). Nothing about either trigger concerns the
login, but that flag is what drives the operator-facing wording, so the refbox reports a
credential problem it has no evidence for.

### The two triggers, and which one matters

Degraded mode is entered from exactly two places:

1. **The portal client could not be built** (`app/mod.rs:2404`). Every failure path inside the HTTP
   client's constructor is TLS-related — certificate-store and TLS-version problems, e.g. *"zero
   valid certificates found in native root store"*. A Raspberry Pi with a broken system
   certificate store is exactly this. **This is the realistic trigger.**
2. **The retry-queue file could not be read** in the config directory *and* in the system temp
   directory (`app/mod.rs:2433`). Both must fail. Near-unreachable.

Note for future readers: two comments in `app/mod.rs` (at `:2389` and `:1112`) claim trigger 1 is
"only possible on a bad https-only config". That is wrong — the https-only setting is a stored
flag enforced per request, so such a configuration builds a client successfully and fails each
call instead. Those comments are **not** corrected by this work; they are recorded here so the
next reader is not misled.

### How narrow is this?

Narrower than the audit report implies, and the narrowing is what makes this a small fix rather
than an urgent one. The portal dot is only displayed when the portal is in use **and** an event is
linked (`app/mod.rs:5909-5920`). With no linked event there is no dot, no false row, and no greyed
REFRESH — the defect is invisible.

There are two realistic paths to it, and **the second is the more reachable one** — found by Eric's
question during the walkthrough, not by this design:

> **Portal path.** A Pi that was linked to an event recently. Its saved link note is restored at
> startup with no network required (`app/mod.rs:2549`), so an event *is* linked. That day the
> certificate store is broken, so the client fails to build and the refbox starts degraded.

> **Custom-site path — more reachable.** A refbox pointed at a third-party site adopts its event
> straight out of the saved URL at startup (`app/mod.rs:2584-2585` → `adopt_custom_event`,
> `app/mod.rs:1183`), with no network and no client. Verified live: the log shows
> `Adopted custom site event events/1234-A` with the client forced absent and the portal link note
> deleted.

The difference matters. The portal path needs a link note **less than 120 hours old**; the custom
path needs only a URL sitting in the config file, which never expires. So a refbox configured for a
third-party site with a broken certificate store shows this red dot **on every launch,
indefinitely**.

### What the operator sees on that path

- A red portal dot.
- On the detail page: **"Access token expired — tap to re-login"**
  (`translations/en-US/refbox.ftl:429`).
- REFRESH greyed out on the game-info screen (`view_builders/game_info.rs:33,45`).
- At game end, a red banner: *"Connection issue detected. Score will still be queued — find an
  admin to resolve."*
- **If they obey the instruction:** the login keypad marks the request as sent
  (`app/mod.rs:4268`) and calls a function that, with no client, returns nothing at all
  (`app/mod.rs:991-1011`). No success, no error, no timeout. They enter the code from the portal
  website and nothing whatsoever happens. The comment at `app/mod.rs:4288` says the reply "will
  replace this once the network request completes" — it never completes.

That dead-end is the harm being fixed: at poolside, the operator burns time on a login that cannot
work, when what they need to know is that results are not uploading.

### What is NOT wrong (checked, and contrary to the audit report)

The report escalates this to a "permanently green" indicator: re-login clears the flag, and in
degraded mode nothing can set it again, so the dot supposedly goes green forever while nothing
uploads. **That escalation cannot occur on the realistic trigger.** Clearing the flag requires a
successful re-login, a re-login requires a client, and the realistic trigger *is* having no client
(`app/mod.rs:991-1011`; a client cannot appear mid-session either — `repoint_client` at
`app/mod.rs:1109-1119` bails out and says a restart is needed).

And even where it is reachable, "permanently green" is overstated: the indicator is recomputed
every second by a UI tick that needs no background task (`app/mod.rs:6242-6245`), so green lasts
only until the first game is queued — then yellow, then red 30 minutes later.

This design therefore addresses the wrong-message half only. The escalation needs no separate work
because the fix removes the flag it depends on.

---

## Goal

A system fault reports itself as a system fault. Specifically:

- Keep the red dot. Something *is* wrong and the operator must know.
- Stop claiming the access token expired.
- Stop pushing the operator into a re-login that cannot send a request.
- Stop greying out REFRESH for a reason that is not true.
- Tell the operator the one thing they can act on: results are not uploading.

## Non-goals

- Not hardening the false-green escalation separately — the fix removes its precondition.
- Not correcting the two misleading "bad https-only config" comments (recorded above).
- Not touching ADR 011's missing failure counter (finding 5) or the unreachable yellow state
  (finding 4).
- Not changing the queue-I/O fallback ordering or the degraded-mode design itself.
- Not making the portal work in degraded mode. It cannot; the point is to say so honestly.

---

## Design

### Approach chosen

A **separate flag for "the portal subsystem never started"**, rather than reusing the existing
`connection_problem` flag.

Reusing `connection_problem` was the audit report's suggestion and was rejected for two reasons:
it would report a connection fault when the trigger can be a local disk failure, and it draws no
detail row at all — trading a wrong message for a blank page. A third option (collapsing both
flags into a single "reason the dot is red" value) was rejected as scope creep: it rewrites
behaviour that is currently correct and tested.

### Changes

| # | Where | Change |
|---|-------|--------|
| 1 | `portal_manager/mod.rs` | New field `startup_problem: bool`. Set `true` **only** by `new_degraded()`; `false` in every other constructor. `new_degraded()` no longer sets `token_known_problem`. |
| 2 | `portal_manager/mod.rs` | `recompute_indicator` treats `startup_problem` as a red cause, alongside `needs_attention()` and `connection_problem`. `token_expired` stays driven by `token_known_problem` alone, so it is now `false` in degraded mode. |
| 3 | `portal_manager/mod.rs` | New `DetailRow::StartupFailed`. `detail_rows()` emits it first when `startup_problem` is set (the position the token row used to hold). |
| 4 | `view_builders/portal_detail.rs` | Render `StartupFailed` as a **non-tappable** red strip, mirroring the existing `RecentSuccess` container shape — there is nothing for the operator to tap. |
| 5 | `translations/` ×15 locales | New key `portal-row-startup-failed`. English, literally: **"Connection unavailable — results will not upload"** |
| 6 | `app/mod.rs:3556` | Complete the existing REFRESH guard: it already refuses to spin when no event is linked; it must also refuse when there is no client. |

### Why change 6 is required, not optional

It is not a nicety — without it this fix introduces a new bug. Today REFRESH is greyed out in
degraded mode because `token_expired` is true. Change 2 makes it live again. In a no-client
session with a restored link, pressing it sets the spinner and calls `request_schedule`, which
returns nothing when there is no client — so the button sticks on "Refreshing…" indefinitely.

The guard mirrors the reasoning already written at `app/mod.rs:3556`: *"Only spin the REFRESH
button when there is actually an event to refresh; otherwise nothing would arrive to clear the
flag."* The same sentence applies word-for-word to the client. With the guard, pressing REFRESH in
a no-client session does nothing — which is already the established behaviour when no event is
linked.

### The wording must be source-neutral — this is a rule, not a preference

**The row must never name the Portal.** This design's first draft said *"Portal unavailable"* and
that was wrong: the refbox can be pointed at a third-party site, and the custom-site path above is
the *more* reachable route to this very screen. Naming the Portal there reports the wrong system.

It would also have reintroduced the exact defect **PR #2219 removed on 2026-08-12**, one day before
this work. Every other string in this block is deliberately neutral — `CONNECTION STATUS`,
*"Access token expired"*, *"Connection issue detected."* — while the **key names** stay `portal-*`
by an explicit decision recorded in that branch. A new `portal-*` key is therefore fine; new
Portal-*wording* is not.

Each locale reuses its own established word for "connection", taken from
`portal-advisory-at-game-end`, so the page reads as one vocabulary: `Verbindung`, `Conexión`,
`Connexion`, `Koneksi`, `Connessione`, `接続`, `연결`, `Sambungan`, `verbinding`, `Ligação`,
`เชื่อมต่อ`, `koneksyon`, `Bağlantı`, `连接`.

**Check the values, not the lines.** A `grep` for "portal" over matching lines gives a false pass,
because the key name contains it. Cut the value after the `=` first — that mistake was made and
caught here.

### Flag lifecycle

`startup_problem` has **no setter other than the constructor**. That is deliberate and correct:
neither trigger can heal without a restart. A missing TLS certificate store does not repair itself
mid-session, and no client can appear mid-session. So the red dot stays red for the whole session,
which is the truth.

This also means the fix cannot regress into the false-green behaviour the audit report worried
about: `token_refreshed()` clears `token_known_problem`, and after this change degraded mode never
sets that flag.

---

## Acceptance criteria

What the human can observe or run to confirm this worked.

**Automated (tests that fail before the change, pass after):**

1. A degraded manager reports the indicator as **Red**.
2. A degraded manager reports `token_expired` as **false**.
3. A degraded manager's detail rows contain `StartupFailed` and **do not** contain `TokenExpired`.
4. A non-degraded manager with a genuine token rejection still reports `token_expired` true and
   still shows the `TokenExpired` row — i.e. the real token path is untouched.

Each guard must be mutation-tested: delete the fix, confirm the test fails, restore. A test that
passes both ways is worthless here, and that is precisely how the original defect survived.

**Observable at the machine:**

5. Start the refbox so that the portal client cannot be built, with an event linked. The portal
   dot is red; the detail page reads "Portal unavailable — results will not upload"; there is no
   "tap to re-login" row; the REFRESH button on game info is **not** greyed out; pressing REFRESH
   does not leave the button stuck on "Refreshing…".
6. Normal operation is unchanged: with a working portal, the dot behaves exactly as before.

**How to produce the failure for step 5 without breaking the machine:** run the refbox against a
throwaway config directory via `XDG_CONFIG_HOME`, so the real portal link is untouched. The
degraded path itself may be easier to reach through a temporary forced-`None` client in a scratch
build than by breaking a real certificate store; if a genuine trigger cannot be produced safely,
say so plainly rather than claiming the walkthrough passed.

---

## Testing notes

- Tests 1–4 are unit tests in `portal_manager/mod.rs`, alongside the existing
  `new_degraded_indicator_has_token_known_problem_and_no_spawned_task` test — **which must be
  renamed and rewritten**, since it currently asserts the very behaviour being removed.
- The REFRESH guard (change 6) lives in `update()` and may not be unit-testable in this codebase's
  current shape. If it is not, say so and verify it in the walkthrough rather than claiming
  coverage that does not exist.

## Risk

Low. One crate, no shared types, no wire format, no state machine, no embedded code. Per
`.claude/rules/plan-execution.md` this is the **lean** process. The only behaviour reaching
existing healthy installations is that a degraded startup now shows a different message and leaves
REFRESH live; no path that works today stops working.
