# 011 — Portal Health Indicator, Per-Game Confirmation, Retry Queue

**Date:** 2026-04-19
**Status:** proposed

## Context

At the 2026-04-18 tournament, a block of games was reported as
missing from the UWH Portal — scores appeared in the refbox at the
time of play but never reached the portal. The operator had no way
to notice this during the tournament — submission is silent today.

A detailed log analysis (see ADR 013) later established that the
immediate cause of those specific missing games was different from
what was first suspected: the refbox cold-started mid-tournament
and the operator replayed earlier games rather than advancing to
the next scheduled one, so games 10-12 never reached
`handle_game_end()` inside the refbox at all. That separate
problem is addressed in ADR 013.

The investigation did, however, surface a real architectural
weakness in `refbox/src/app/mod.rs` that is worth fixing
independently. When a game ends, `handle_game_end()` calls
`post_game_score()` and `post_game_stats()` as fire-and-forget
tasks. Each of those tasks:

- logs the error if the HTTP call fails, and
- returns `Message::NoAction` — the UI is never told that anything
  went wrong.

There is no retry, no persistence across crashes, and no visible
indicator of portal health. A transient network drop during the last
30 seconds of a game would silently discard that game's score. The
only artefact is a line in the Windows log file at
`%LOCALAPPDATA%\uwh-refbox-logs\refbox-log.txt` which the operator
would never look at during play. A future incident — expired token,
flaky network, portal outage mid-tournament — would hit exactly
that silent-failure path. This ADR addresses that weakness
proactively rather than waiting for it to bite.

Design decisions made during brainstorming:

- **Shape of the indicator.** A square clickable tile on the left end
  of the top time banner, containing the UWH Portal compact logo
  above a coloured status dot. Tile background is `LIGHT_GRAY` so it
  reads as a tappable panel against the banner's medium-gray
  background.
- **Colour meanings.** Green = last portal exchange succeeded.
  Yellow = a check is in flight or a recent call degraded. Red =
  a call failed and something needs attention (queued retry,
  rejected submission, or expired token).
- **Per-event overlays.** A green checkmark briefly replaces the dot
  for ~10 seconds after a successful submission, then snaps back.
  A red exclamation mark persists on top of the dot as long as at
  least one item needs attention. Tapping the tile opens the detail
  page.
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

Introduce a **Portal Health Indicator** — a clickable tile on the
left end of the time banner (UWH Portal logo above a coloured status
dot) backed by a new `portal_manager` module that tracks submission
state, retries failures from disk, and surfaces problems to the
operator.

### Status indicator

- A square clickable tile is rendered at the left end of the top
  time banner on every page that already shows the banner. The tile
  holds the UWH Portal compact logo above a coloured status dot, on
  a `LIGHT_GRAY` background so the tile reads as tappable.
- The dot's three base colours:
  - **Green** — last portal exchange succeeded, queue is empty, token
    is valid.
  - **Yellow** — a health check is currently in flight, or the most
    recent call was degraded (slow but succeeded).
  - **Red** — at least one item needs attention: a queued retry has
    failed, a submission was rejected by the portal, or the token
    has expired.
- Overlay icons:
  - **Green checkmark** replaces the dot for ~10 seconds immediately
    after a successful score submission, then snaps back to the
    plain dot.
  - **Red exclamation mark** persists on top of the dot as long as the
    detail page would have anything in its "attention needed" list.
    It clears only when every attention item is resolved.
- Tapping the tile (any state) opens the Detail Page.

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

> **Status:** All five questions below have since been resolved
> during spec-writing on 2026-04-19. The resolutions are captured in
> `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`.
> The original question text is preserved here for historical context.

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

## Amendments

### 2026-04-21 — Conflict handling refined after API verification

After verifying `uwh-common`'s portal client code, we confirmed that
`post_game_scores()` returns `Result<(), Box<dyn Error>>` and collapses
every non-success HTTP response (409 Conflict, 401 Unauthorised, 500,
network failure, etc.) into a single generic error. No portal-side
score values come back on conflict. Distinguishing a conflict from
any other failure would require modifying `uwh-common`, which is
ruled out by the standing constraint not to change that crate.

Given that, the conflict-resolution flow originally described above is
refined as follows:

- **No dedicated `Conflict` state.** All non-success submission
  outcomes become `Pending` and auto-retry in the background.
- **Yellow health state** replaces the word "orange" used earlier in
  this ADR — the queue has items retrying; informational only. The
  theme already defines a `YELLOW` colour constant, so no new colour
  is needed.
- **Red health state** — triggered by (a) token expired (detected by
  the periodic `verify_token` health check, since the score-submission
  error type cannot be introspected) or (b) a queued item has been
  continuously retrying for 30 minutes without success. Both
  indicate operator intervention is needed.
- **Single "attention" action page** (replaces the separate conflict
  and pending action pages) — offers `FORCE THIS GAME RESULT` (retry
  with `force=true`, wins any portal-side disagreement) and
  `DISCARD THIS SUBMISSION` (remove from queue; whatever the portal
  currently has stands). No `KEEP REFBOX VALUE` / `KEEP PORTAL VALUE`
  buttons; no portal-side values displayed.
- **No changes to `uwh-common`.** The entire feature is built on the
  existing portal-client API surface.

The operator workflow under this refinement is: a conflict (real or
apparent) just looks like a stuck pending item. After 30 minutes of
continuous failure it escalates to red. The operator taps it, chooses
`FORCE` to overwrite the portal or `DISCARD` to keep whatever the
portal has. If they want to see the portal-side value before
deciding, they look at it on the portal's web UI separately.

The original design (Option A) is preserved as historical context
in the companion spec. The implementation plan at
`docs/superpowers/plans/2026-04-19-portal-health-indicator.md`
reflects this refined design.

### 2026-04-22 — Translation coverage and verification environment

Two implementation questions surfaced during an audit of the in-flight
plan. Both are recorded here so the plan can be updated to match.

**Translation coverage.** Every user-visible string on the new portal
detail page, the attention-action page, and the token-expired action
page is translated through the Fluent (`refbox/translations/`) system.
In addition to the keys listed in Task 21 of the implementation plan,
two body-paragraph strings were missed and must be added:

- the attention-action page body text
  (`"This result has not been accepted by the UWH Portal after N
  attempts. Refbox value: B-W"`), and
- the token-expired page body text
  (`"The UWH Portal login has expired. Queued scores cannot be sent
  until you log in again. Tap GO TO LOGIN to re-authenticate."`).

Fluent key parametrization must match what the code actually computes.
Specifically, `portal-summary-issues` is the simple form
`"PORTAL — ISSUES"`, with no `Last OK { $duration } ago` suffix — the
code does not track that value, and adding it is out of scope for this
ADR. All other plan keys are retained as listed.

**Verification environment.** End-to-end manual verification (Task 22
of the plan) runs against a local instance of the UWH Portal API as
the primary environment, with `dev.uwhportal.com` retained as a
fallback option. The local setup is the uwh-portal API process running
on port 5000 against a local database, per the uwh-portal project's
development guide, with a test event created in that local database.
The operator points the refbox at this local instance with
`UWH_PORTAL_URL_OVERRIDE=http://localhost:5000`.

Localhost is preferred because it gives the operator full control over
the test data: the Scenario 3 "forced conflict" test becomes trivial
(the operator can edit the test event on the local web UI to induce a
409 on the next submission), no cleanup of real portal data is
required afterward, and the whole verification is fully deterministic.
If the local setup proves impractical for any scenario, the hosted
`dev.uwhportal.com` environment is used instead, following the same
cleanup discipline (delete any test scores and the test event after
the run).

The implementation plan at
`docs/superpowers/plans/2026-04-19-portal-health-indicator.md`
will be updated to reflect both decisions — Task 21's key list is
corrected for the two new body keys and the simplified
`portal-summary-issues`, and Task 22's verification steps are rewritten
around the local portal instance with a fallback note for
`dev.uwhportal.com`.
