# 011 — Portal Health Indicator, Per-Game Confirmation, Retry Queue

**Date:** 2026-04-19
**Status:** proposed

## Context

A tournament operator has reported games where the score appeared in
the refbox but never reached the UWH Portal. The operator had no way
to notice this during the tournament — submission is silent today.

Investigation of `refbox/src/app/mod.rs` identified the cause. When a
game ends, `handle_game_end()` calls `post_game_score()` and
`post_game_stats()` as fire-and-forget tasks. Each of those tasks:

- logs the error if the HTTP call fails, and
- returns `Message::NoAction` — the UI is never told that anything
  went wrong.

There is no retry, no persistence across crashes, and no visible
indicator of portal health. A transient network drop during the last
30 seconds of a game silently discards that game's score. The only
artefact is a line in the Windows log file at
`%LOCALAPPDATA%\uwh-refbox-logs\refbox-log.txt` which the operator
would never look at during play.

Related confirmed non-issues (ruled out during investigation):

- **"End game and apply" on a game-number change** was flagged as a
  suspect but is working as designed. The confirmation dialog offers
  "Keep current game" as a sibling option, so choosing "End" is an
  explicit acknowledgement that the current game was started in
  error.
- **`using_uwhportal` starting `false` at launch** is real, but the
  reporting operator knew about it and the game-config info confirmed
  the portal was connected.

So the remaining candidate cause — silent submission failure during
or after the game — is what this ADR addresses.

Design decisions made during brainstorming:

- **Shape of the indicator.** A small coloured dot on the left side
  of the top time banner. Not a UWH-Portal logo. The operator wanted
  a simple "is it OK" signal, not branding.
- **Colour meanings.** Green = last portal exchange succeeded.
  Yellow = a check is in flight or a recent call degraded. Red =
  a call failed and something needs attention (queued retry,
  rejected submission, or expired token).
- **Per-event overlays.** A green checkmark briefly replaces the dot
  for ~10 seconds after a successful submission. A red exclamation
  mark persists on top of the dot as long as at least one item needs
  attention. Tapping the dot/mark opens the detail page.
- **Detail-page layout.** Matches the Select Event scrollable page —
  4/5-width list on the left with scroll up/down arrows, 1/5-width
  side column with only a `BACK` button at the bottom. Each list row
  is a coloured button (red = conflict, orange = pending retry,
  green = recently submitted, gray = neutral). The status summary
  sits in the list's title slot.
- **Actions happen on follow-on pages.** Tapping a row opens a page
  scoped to that one game with the appropriate choice buttons. There
  is no global "Verify Now" action bar on the detail page itself.
- **Architectural approach.** A new `portal_manager/` module inside
  `refbox/src/`, paralleling the existing `tournament_manager/` and
  `sound_controller/` modules. The module owns the retry queue, the
  background health-check timer, and the state machine that emits
  `Message`s to the UI. Rejected alternative: extending the portal
  client in `uwh-common` — rejected because `uwh-common` must stay
  `no_std` compatible and the retry/persistence logic is inherently
  `std`.

## Decision

Introduce a **Portal Health Indicator** — a dot + overlay icon on the
time banner backed by a new `portal_manager` module that tracks
submission state, retries failures from disk, and surfaces problems
to the operator.

### Status indicator

- A small coloured circle ("dot") is rendered at the left side of the
  top time banner on every page that already shows the time banner.
- Three base colours:
  - **Green** — last portal exchange succeeded, queue is empty, token
    is valid.
  - **Yellow** — a health check is currently in flight, or the most
    recent call was degraded (slow but succeeded).
  - **Red** — at least one item needs attention: a queued retry has
    failed, a submission was rejected by the portal, or the token
    has expired.
- Overlay icons:
  - **Green checkmark** replaces the dot for ~10 seconds immediately
    after a successful score submission, then fades back to the plain
    dot.
  - **Red exclamation mark** persists on top of the dot as long as the
    detail page would have anything in its "attention needed" list.
    It clears only when every attention item is resolved.
- Tapping the dot (any state) opens the Detail Page.

### Detail page

Same chrome as every other page: time banner on top, timeout ribbon
on bottom. Between them:

- **List area (4/5 width).** Scrollable list of game submissions.
  - The list's title slot shows the status summary (e.g.
    `PORTAL — RECENT ISSUES · Last OK 4 min ago`) with a coloured
    dot matching the overall health.
  - Each row is a coloured button:
    - **Red** — conflict (portal and refbox disagree on score).
    - **Orange** — pending retry (queued, will retry automatically).
    - **Green** — recently submitted successfully (informational).
    - **Gray** — neutral state (e.g. token check).
  - Only red and orange rows are tappable; green rows are visual only.
  - Scroll up/down arrows sit in a narrow column on the right, same
    as Select Event.
- **Side column (1/5 width).** A single `BACK` button pushed to the
  bottom by `vertical_space`, matching Select Event exactly.

### Per-item action pages

Tapping an attention row opens a full page scoped to that one item.

- **Red (conflict) row** → page offering `KEEP REFBOX VALUE` vs
  `KEEP PORTAL VALUE` and a `BACK` button. Confirming re-submits with
  `force=true` on the `post_game_scores` call.
- **Orange (pending) row** → page offering `RETRY NOW` vs `DISCARD`
  and a `BACK` button.
- **Green (recent) row** is not tappable; if this changes in future,
  the page would be read-only details.

All action pages use the same chrome (time banner top, timeout ribbon
bottom) and the standard button colours already used by the refbox.

### `portal_manager` module

A new module at `refbox/src/portal_manager/` owning:

- **A persisted retry queue** keyed on `(event_id, game_number)`. On
  every game end, a record is written to disk immediately and then
  the submission is attempted. The record is only removed after the
  portal confirms receipt. Storage: a JSON file in the same directory
  `confy` already uses for the refbox config, so that it survives a
  crash or restart.
- **A dual-interval background health-check timer.** Default cadence
  is **5 minutes** while green. When the state becomes yellow or red,
  cadence drops to **15 seconds** until green again. The timer resets
  on every successful portal interaction (health check or score
  submission) so that a just-sent score is not followed by a redundant
  health check.
- **The health check itself** reuses the existing `verify_token`
  endpoint (`GET /api/events/{event}/access-keys/verify`) already
  called at login. A 200 OK means green. A non-200 or a timeout means
  yellow (single failure) or red (repeated failure or token
  rejection).
- **State machine outputs** emitted as iced `Message`s that the main
  app consumes to update the indicator and the detail-page list.

The queue's exact file format and the state-machine shape are
implementation details; this ADR fixes only the observable behaviour
and the module boundary.

### Conflict resolution

The current `post_game_scores` call is always made with `force=false`.
After this change:

- First submission attempt still uses `force=false`.
- If the portal rejects because a score already exists that
  disagrees, the item becomes a **conflict** (red row on the detail
  page). The operator opens the row's page, chooses refbox or portal
  value, and the refbox resubmits with `force=true` **only if the
  operator picked "refbox"**. Picking "portal" removes the row from
  the queue without a resubmit — the portal wins.
- This also covers the case where someone edits the score directly in
  the UWH Portal web UI: the refbox's next submit attempt will
  collide, surface as a conflict, and the operator can explicitly
  accept the portal's edit.

### What is **not** changing

- `uwh-common`'s portal client API is not changing shape. The retry
  and queue logic wraps it; the HTTP surface stays the same.
- LED-panel output, overlay output, and wireless-remote behaviour are
  untouched.
- Game logic, clock, and rules are untouched.
- No change to the wire format between refbox and overlay.

## Open design questions (to resolve during implementation)

- **Exact dot placement in the time banner.** Two options: (a) inject
  the dot into the existing `make_game_time_button` row, or (b) add a
  thin band above/below the time banner that the dot lives in.
  Option (a) keeps the banner chrome identical to today but requires
  editing a widely-reused view helper. To be decided during
  implementation with a quick mockup pass.
- **"Recent" section retention policy.** The detail page currently
  shows green rows for recent successes. Open choices: keep last N
  (e.g. N=5), age out after X hours, or clear on app restart.
  Leaning toward "clear on restart + keep last 5 while running" but
  not committed.
- **Expired-token recovery flow.** When a token has expired (distinct
  from a transient failure), the detail page needs a way to push the
  operator back through the portal-login screen. Likely surface: a
  red row on the detail page whose per-item action page is "go to
  login." To be finalised during implementation.
- **End-game-while-red behaviour.** Current proposal: no special
  warning at game-end time. The score is queued as normal and the
  exclamation mark stays up. Alternative would be a modal blocker at
  game end if portal is red, but that risks interrupting a referee
  during a busy finish. Default is "queue silently, indicator stays
  red."
- **Queue file format and schema versioning.** Implementation-level
  detail. Start with a plain JSON array of submission records, keyed
  on `(event_id, game_number)`, with a schema version field so that
  future changes can be migrated.

## Consequences

**Becomes easier:**

- Operators can tell at a glance whether the tournament is being
  recorded correctly. A red mark at the top of the screen demands
  attention and cannot be missed.
- Transient network failures no longer silently lose a score. The
  retry queue survives restarts and crashes, so closing the refbox
  overnight and reopening at the next day's session picks up pending
  items.
- Score edits made in the UWH Portal web UI during play are
  surfaced as conflicts rather than silently overwritten on a resubmit.
- The operator can manually "verify connection now" by opening the
  detail page — useful when they're about to start a game after a
  long break.

**Becomes harder / constrained:**

- `refbox` gains a persistent on-disk side channel (the queue file).
  Backup and migration tools have one more file to consider.
- Every page that shows the time banner now also shows the health
  indicator, so the time-banner view helper becomes a little more
  complex and gains a new message dependency.
- The portal is now polled even when nothing is happening (5-minute
  green cadence). This is a tiny amount of traffic per refbox but it
  exists where before there was none.
- Token expiration becomes a visible, user-surfaced event rather than
  a silent degradation — this is a positive change but requires a new
  login-recovery flow that didn't exist before.

**Scope:**

- `refbox` — new `portal_manager/` module; changes to `app/mod.rs` so
  game-end routes through the manager; new view builder for the
  detail page; new view builders for the three per-item action pages;
  time-banner helper extended to carry the health dot; new messages in
  `app/message.rs`.
- `uwh-common` — no change to the portal client signature. The
  `post_game_scores(force: bool)` flag already exists and is reused.
- `overlay`, `schedule-processor`, `wireless-remote`, LED-panel
  crates, `matrix-drawing`, `fonts` — no change.

## References

- `refbox/src/app/mod.rs` — `handle_game_end()`, `post_game_score()`,
  `post_game_stats()`; the call sites this ADR reroutes through
  `portal_manager`.
- `refbox/src/tournament_manager/mod.rs` — structural model for the
  new `portal_manager/` module (owned state, background tick timer,
  iced `Message` output surface).
- `refbox/src/app/view_builders/list_selector.rs` — the Select Event
  page that the detail page's chrome copies (4/5 list + 1/5 side
  column, scroll arrows column, single bottom button).
- `refbox/src/app/view_builders/shared_elements.rs` —
  `make_scroll_list`, `make_game_time_button`; the widget-level
  building blocks the detail page and the indicator reuse.
- `uwh-common/src/uwhportal/mod.rs` — `verify_token`,
  `post_game_scores(force)`, `post_game_stats`; the HTTP surface the
  retry queue wraps.
- `refbox/src/app/theme/mod.rs` — colour constants used by the
  indicator dot and the coloured row buttons.
- `.superpowers/brainstorm/32014-1776565793/content/detail-page-v3.html`
  — the final detail-page mockup agreed during brainstorming. The
  spec will copy its layout faithfully.
- `memory/feedback_backport_web_is_standard.md` — if the web refbox
  ships an equivalent indicator, that becomes the authoritative
  source and this design is revisited.
