# 011 — Portal Health Indicator, Per-Game Confirmation, Retry Queue

**Date:** 2026-04-19
**Status:** accepted (verified by Unit 7 audit 2026-05-15)

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

### 2026-04-23 — Dormant until event linked

During Task 22 walkthrough prep, operator review surfaced that the
health tile was rendering even before any portal event had been
linked. The tile served no purpose in that state: no submission
path, no retry queue traffic, no token to check. Worse, the
background `verify_token` loop's URL construction with a stale
saved event_id from a previous session would return a 404 response
against the dev portal, cluttering the log without any
user-actionable signal.

This amendment refines the tile-visibility rule originally stated
in the Decision section's "Status indicator" block:

- **The tile is only rendered when a portal event is currently
  linked** (that is, the operator has selected an event through
  the refbox's portal-login flow and the selection is live in
  `current_event_id`). When no event is linked, the time banner
  falls back to its pre-feature layout — identical to what the
  operator saw before this feature existed.
- **The feature activates** when an event is picked through the
  refbox's portal-login flow, and **deactivates** when the operator
  unlinks or changes event. The round-trip through unlink-and-relink
  is supported without restarting the app.
- **The background health-check task already short-circuits**
  `verify_token` when no event id is set (returning `Ok` without
  making a network call), so no further changes to the background
  side are required by this amendment.
- **The confirm-score "portal red" advisory banner** is also
  suppressed when no event is linked — without a submission path
  there is no meaningful advisory to show.

The original phrasing in the Decision section's "Status indicator"
bullet — *"a square clickable tile is rendered at the left end of
the top time banner on every page that already shows the banner"*
— is superseded by this amendment: read it with the added
qualifier *"on every banner-bearing page **once a portal event has
been linked**"*.

**Out of scope for this amendment:** behaviour when the retry
queue has items but no event is linked (e.g. after a
mid-tournament restart without re-linking). The queue persists
each item with its own `event_id` so the retry loop can still
fire for those items; the tile, however, stays hidden under this
rule, so any stuck-item attention would only surface after the
operator relinks to an event. This edge case is rare in practice
and the operator's next re-link restores tile visibility.

Code change lands on `feat/refbox/portal-health-indicator` at
commit `34b0b17` — `feat(refbox): make portal health feature
dormant until event linked`. The Deviations log entry for
Task 22 captures the same change on the plan side.

### 2026-04-23 — Remove the 10-second green-checkmark overlay

During Task 22 walkthrough prep, after directly witnessing the
overlay behaviour, operator review concluded the
green-checkmark-on-dot flash is redundant: the portal detail page
already lists each recent submission as a green "Submitted N min
ago" row, accessible with a single tap on the tile. A transient
10-second overlay on the tile communicates the same "a submission
just landed" signal, but in a weaker and more ephemeral form, and
costs additional state (a deadline timestamp, a periodic tick that
re-renders the tile when the deadline expires).

This amendment supersedes the following bullet from the Decision
section's "Overlay icons" block:

> *Green checkmark replaces the dot for ~10 seconds immediately
> after a successful score submission, then snaps back to the plain
> dot.*

The green-checkmark overlay is removed. The operator's confirmation
path for "did my score land?" is now exclusively the detail page,
which the tile already opens on tap. The tile's three base colours
(Green / Yellow / Red) still convey current health at a glance —
after a successful submission, the tile simply returns to Green.

The **red exclamation overlay** on top of the red base dot (the
`AttentionNeeded` overlay) is retained — it surfaces a persistent
"needs attention" state that has no other at-a-glance equivalent.
Only the transient success overlay is removed.

The in-memory `recent_successes` ring that populates the detail
page's green rows is unchanged. Only the overlay-timer machinery
and its variant on `OverlayState` are removed.

Code change lands on `feat/refbox/portal-health-indicator` at
commit `5038231` — `refactor(refbox): remove 10-second
green-checkmark overlay on successful submit` — which also removes
the orphaned `check_circle.svg` asset and two now-moot unit tests.
The Deviations log entry for Task 22 captures the same change on
the plan side.

### 2026-05-16 — Indicator dormant when Using-UWH-Portal is off

Operator review on 2026-05-16 surfaced that the portal health
indicator stayed visible even when the operator had set
"Using UWH Portal" to NO (provided an `current_event_id` was
still linked from a prior session). This contradicts the spirit
of "use of the UWH Portal feature is gated by the operator toggle".

The 2026-04-23-dormant amendment established the rule "indicator
hidden when no event is linked" (`current_event_id == None`). This
2026-05-16 amendment adds a second necessary condition:

**Indicator visibility now requires both:**
- `self.using_uwhportal == true` (the operator has the feature on), AND
- `self.current_event_id.is_some()` (an event is linked)

When either condition fails, `portal_indicator: None` flows to the
view builders and the time banner falls back to the pre-feature
layout — same behaviour as the 2026-04-23-dormant amendment, just
triggered by a broader gate.

Code change lands on `feat/refbox/portal-subsystem-dormancy`:
the conditional in `app/mod.rs::view()` (around line 3055) is
extended from `self.current_event_id.as_ref().map(...)` to
`if self.using_uwhportal { self.current_event_id.as_ref().map(...) } else { None }`.

The pairing with ADR 017's dormancy contract (no fetches when
toggle is off) makes the indicator's behaviour internally
consistent: portal subsystem is invisible AND idle whenever the
operator has not opted in.

## Verified by Unit 7 audit (2026-05-15)

### Audit scope

- **AUDIT-PLAN.md section:** Unit 7 — PR #761 portal health indicator
- **Audit branch:** `audit/refbox/portal-health` (local-only; cut from
  `feat/refbox/portal-health-indicator` HEAD at `0a5cc2e`, not from
  master). 5 audit commits ahead of the feature branch tip.
- **Commit range audited:** `3ce6bdd..0a5cc2e` on
  `feat/refbox/portal-health-indicator` (46 in-scope commits; the
  Renovate bump `089c98d` was out of scope).
- **Walkthrough date:** 2026-05-15
- **Walkthrough environment:** `https://api.dev.uwhportal.com` (the
  correct dev API URL — separate `api.` subdomain from the
  web frontend; see "What was not verified" below for the URL
  discovery process).

### Verified decisions

For each ADR section and amendment, the catalog entries that
realise and verify it. Catalog detail in `AUDIT-PLAN.md` Unit 7
"Behaviour catalog" subsection.

- **Status indicator (Decision section):** verified by B7.A9 (Yellow
  rule), B7.A10 (Green rule), B7.C1 (`PortalIndicatorState` threaded
  into `make_game_time_button`), B7.C2 (tile renders on time banner),
  B7.C5 + B7.C6 (sizing fixes for standard and tall banners),
  B7.D1 (compact UWH Portal logo asset).
- **Detail page (Decision section):** verified by B7.A20 (page row
  ordering: token, stuck-oldest-first, pending-oldest-first,
  successes-newest-first), B7.C7 (view-builder), B7.C8 (action-routing
  message variants), B7.C18 (`make_scroll_list` + `PORTAL_DETAIL_LIST_LEN`
  of 4), B7.D6 (public-API tests).
- **Per-item action pages (Decision section, refined by amendment
  2026-04-21):** verified by B7.A13 (public mutation API — `enqueue`,
  `force_submit`, `discard`, `token_refreshed`), B7.C9
  (`ClosePortalAttentionAction` BACK scope), B7.C10 (attention action
  page view-builder), B7.C27 (standard-layout alignment).
- **`portal_manager` module (Decision section):** verified by Group A
  entries B7.A1 (module scaffold) through B7.A25 (background task
  with `PortalTaskIo` trait + `spawn` + `run_task` + `attempt_item`).
- **Conflict resolution (Decision section, refined by amendment
  2026-04-21):** verified by B7.A8 (Red rule when token-problem or
  stuck), B7.A13 (force/discard public API), B7.A24 (retry eligibility:
  not stuck AND ≥15s since last attempt).
- **Amendment 2026-04-21 (single attention page; Yellow replaces
  Orange):** verified by B7.A4 (`QueuedItem` shape with force flag),
  B7.A7 (`is_item_stuck` and 30-minute STUCK_THRESHOLD), B7.A24
  (retry-eligibility), B7.C10 (attention page).
- **Amendment 2026-04-22 (translation coverage and verification
  environment):** verified by B7.C13 (Fluent externalization of
  indicator + detail + attention strings), B7.C23 (mode-aware sport
  prefix in portal strings), B7.E1 (`UWH_PORTAL_URL_OVERRIDE` env
  var). Walkthrough environment was `https://api.dev.uwhportal.com`
  per the URL discovery — see "What was not verified" below.
- **Amendment 2026-04-23 (dormant until event linked):** verified by
  B7.A19 (`on_token_status(valid)` handler), B7.B6 (dormant gate at
  view-data construction), B7.C14 (detail-page suppression when no
  event linked), B7.C15 (confirm-score advisory suppression when no
  event linked). Walkthrough Scenario 5 live-confirmed the tile-hidden
  assertion.
- **Amendment 2026-04-23 (remove 10-second green-checkmark overlay):**
  verified by B7.A21 (dead-code removal pass dropping 8 items when
  the scaffolding `#[allow(dead_code)]` was lifted, including the
  overlay-timer machinery and `OverlayState::RecentSuccess`),
  B7.C16 (UI side removal). Note: the amendment's intent to *retain*
  the red exclamation overlay was contradicted by audit-window
  commit `0fb9ed2`; see Supersessions below.

### Supersessions

Operator-decided in Task 5 (2026-05-15) review:

- **C11 supersedes the implied single-tap discard.** Amendment
  2026-04-21 names the `DISCARD THIS SUBMISSION` button but does not
  mandate a tap pattern. The audit-window code requires two taps
  (the first changes the button text to "TAP AGAIN TO CONFIRM
  DISCARD"; the second fires the discard). Operator confirmed the
  two-tap safety in walkthrough Scenario 4. Read the amendment's
  single button name as compatible with the two-tap gesture.
- **C17 supersedes amendment 2026-04-22's per-state title
  expectation.** Amendment 2026-04-22 simplified `portal-summary-issues`
  to `PORTAL — ISSUES` with no `Last OK …` suffix, but per-state
  distinctions (different titles for green/yellow/red) were implied
  in ADR 011's "Detail page" Decision section ("e.g. `PORTAL — RECENT
  ISSUES · Last OK 4 min ago`"). The audit-window UX pass collapsed
  the title to a single static string regardless of state. Operator
  confirmed the simplification — the dot color on the tile conveys
  state at-a-glance, so the detail-page title can be neutral.
- **C20 supersedes amendment 2026-04-21's literal button text.**
  Amendment 2026-04-21 named the action buttons `FORCE THIS GAME
  RESULT` and `DISCARD THIS SUBMISSION` (all-caps). The audit-window
  UX pass renamed them to `Retry this game result` and `Discard this
  game result` (sentence case) and added a stored-score display the
  ADR does not mention. Operator confirmed the renamed buttons and
  the stored-score display in Task 5.
- **C26 supersedes amendment 2026-04-23-overlay's "the red exclamation
  overlay is retained" sentence.** The amendment removed the
  10-second green-checkmark overlay but explicitly retained the red
  exclamation overlay ("the AttentionNeeded overlay … is retained").
  Audit-window commit `0fb9ed2` ("simplify portal attention page and
  retire overlay indicator") removed the red exclamation overlay
  anyway. Operator confirmed in Task 5 that the solid red dot is
  sufficient as the at-a-glance attention signal; the red exclamation
  overlay is retired and not restored. Read amendment 2026-04-23's
  "retained" sentence as superseded.

### What was removed during audit

None. The audit produced two surgical fixes (commits `5d2d318` and
`38482fd`) rather than deletions:

- **B7.E2 + B7.B9 — compile-time gate `UWH_PORTAL_SCRAMBLE_TOKEN`**
  (commit `5d2d318`). The runtime gate `cfg!(debug_assertions)` left
  the env-var name string in release binaries; the surgical fix wraps
  the field, the env-var read, and the trigger in
  `#[cfg(debug_assertions)]` so release binaries contain no trace.
  `UWH_PORTAL_URL_OVERRIDE` (B7.E1) was operator-confirmed to stay
  ungated for dev/staging workflows.
- **B7.C19 hybrid partial revert — restore `(attempt N)` suffix**
  (commit `38482fd`). The audit-window UX pass had stripped per-row
  attempts, retry-timer, and stats-only suffixes from the detail page
  along with the underlying `DetailRow::Pending::attempts` field.
  The hybrid revert restores the `attempts` field and the
  `(attempt N)` suffix (operator-actionable info); does NOT restore
  the per-second retry timer (deemed visual noise); does NOT restore
  the stats-only suffix (discovered to be pre-existing dead code —
  the field was always hardcoded `false` even before the strip).

### What was not verified

- **Walkthrough Scenario 3 (induced 409 conflict resolution) is
  `@tested_deferred`.** Operator chose to close walkthrough at 7-of-8
  scenarios on 2026-05-15. The 409 induction requires manually
  editing the score on the dev portal web UI between submissions —
  more involved than the other scenarios. The conflict-resolution
  mechanics are exercised indirectly by Scenario 4 (two-tap discard)
  and the FORCE-button code path (B7.A13 + amendment 2026-04-21).
- **Scenario 2's 30-minute red-escalation threshold was verified at
  a temporary 1-minute substitute, not the 30-minute production
  value.** During walkthrough, the `STUCK_THRESHOLD` constant in
  `refbox/src/portal_manager/mod.rs` was temporarily reduced from
  30 minutes to 1 minute via a working-tree-only edit (reverted before
  any commit; git diff clean post-walkthrough). The same code path is
  exercised at both values; only the wall-clock duration differs.
  The 30-minute value is verified by the existing unit tests under
  `refbox/src/portal_manager/health.rs`.
- **Scenario 5's "no background 404s when unlinked" sub-assertion is
  `@tested_inconclusive`.** The original walkthrough attempt (against
  `https://dev.uwhportal.com` — the web frontend, not the API)
  produced log noise from HTML 404 responses, which conflicted with
  amendment 2026-04-23-dormant's "background task short-circuits
  `verify_token` when no event id is set" claim. With the corrected
  override URL (`https://api.dev.uwhportal.com`), the unlinked state
  was not retested in isolation, so the sub-assertion's verification
  status remains inconclusive. The dormant tile-hidden + relink-
  restores-tile assertions are both `@tested_pass`.
- **Scenario 8's UWR-mode logo is `@tested_deferred`.** Operator was
  in UWH mode throughout the 2026-05-15 walkthrough. The UWR-mode
  logo (B7.C22) is half-finished — the logo asset switches based on
  mode but the URL routing stays UWH-only. Carved out as
  `@redesign-followup` in Task 5 with suggested follow-up branch
  `feat/refbox/uwr-portal-support`.

### Audit reference

- **Per-unit plan:** `docs/superpowers/plans/2026-05-15-audit-unit-7-portal-health.md`
- **Audit-design spec:** `docs/superpowers/specs/2026-05-15-audit-unit-7-portal-health-design.md`
- **Gherkin scenarios:** `refbox/tests/features/portal-health.feature` on `audit/refbox/portal-health` — 8 scenarios, 7 `@tested_pass` + 1 `@tested_deferred`.
- **Cross-branch dependencies:** none. Unit 7 is self-contained — no
  hand-applied commits from other branches (contrast with Unit 5's
  `ed94287`).
- **Findings filed:** 7 entries in `AUDIT-PLAN.md` → Findings backlog
  → From Unit 7 (2026-05-15) covering the degraded-startup semantics
  (A15/A16), the unwired `check_in_flight` flag (A22),
  the `retry_in_secs` test realignment (D12), the catalog-noise
  refactor (D10), the UWR portal support follow-up (C22),
  pre-existing `bool_assert_comparison` test-code clippy errors
  (master-state), and the dev portal API URL documentation.
