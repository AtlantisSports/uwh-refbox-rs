# Audit Unit 7 — Portal Health Indicator walkthrough scenarios.
#
# Oracle: docs/decisions/011-portal-health-indicator.md + its 4 amendments.
# Per-unit plan: docs/superpowers/plans/2026-05-15-audit-unit-7-portal-health.md
#
# Walkthrough environment: local uwh-portal API at http://localhost:5000
# launched per ADR 011 amendment 2026-04-22. Refbox launched with:
#   UWH_PORTAL_URL_OVERRIDE=http://localhost:5000 \
#   WAYLAND_DISPLAY= RUST_LOG=info \
#   cargo run -p refbox
#
# Operator-decided clarifications from Task 5 (2026-05-15):
# - Two-tap discard confirmation kept (B7.C11).
# - Detail-page single static title kept (B7.C17).
# - Attention-page buttons renamed to "Retry this game result" /
#   "Discard this game result" with stored-score display (B7.C20).
# - Red exclamation overlay retirement kept; only the dot conveys state (B7.C26).
# - `(attempt N)` suffix restored on pending rows (B7.C19 hybrid).
# - UWR mode-aware logo deferred to a follow-up branch (B7.C22).

Feature: Portal health indicator
  A clickable tile on the left end of the time banner shows whether the
  refbox is successfully communicating with the UWH Portal. The tile
  appears only when a portal event is linked. The tile's dot color
  conveys state at-a-glance: green (all good), yellow (in flight or
  retrying), red (attention needed). Tapping the tile opens a detail
  page listing recent submissions and any items needing attention.

  Background:
    Given the local uwh-portal API is running at http://localhost:5000
    And a test event exists on the local portal
    And the refbox is launched with UWH_PORTAL_URL_OVERRIDE=http://localhost:5000

  @user_verified @tested_deferred
  Scenario: Green path — successful submission lands silently
    Given the operator has logged in and linked the test event
    And the portal indicator tile is visible on the time banner
    And the tile dot is green
    When the operator ends a game and confirms the score
    Then the score is submitted to the local portal
    And the score appears in the local portal's web UI for that event
    And the portal indicator dot stays green throughout
    And no operator-facing dialog interrupts the flow

    # Session notes:
    # @tested_deferred
    # walkthrough attempt: 2026-05-15 09:38
    # Could not complete on dev.uwhportal.com because the dev URL override
    # (UWH_PORTAL_URL_OVERRIDE=https://dev.uwhportal.com) hit the web frontend
    # and returned HTML for every API call (verify_token failed with HTML 404;
    # event list "Loading..." hung). The actual dev API URL path is unknown.
    # Deferred until a local uwh-portal instance is booted, or the correct dev
    # API URL is identified. See Findings backlog From-Unit-7 #7.

  @user_verified @tested_deferred
  Scenario: Network drop — indicator escalates green → yellow → red
    Given the portal indicator dot is green
    When the local portal API is stopped (or port 5000 is blocked)
    Then the next background health check fails
    And the indicator dot transitions to yellow within one health-check cycle
    And if failures persist for 30 continuous minutes, the indicator dot escalates to red
    And no exclamation overlay appears on top of the red dot (per Task 5 C26 decision — dot color alone is the at-a-glance signal)

    # Session notes:
    # @tested_deferred (requires linked event + 30-minute escalation window)
    # walkthrough attempt: 2026-05-15 — could not complete without linked event

  @user_verified @tested_deferred
  Scenario: Induced 409 conflict — operator chooses Retry or Discard
    Given the portal indicator dot is red because a queued submission has been stuck for 30+ minutes
    When the operator taps the portal indicator tile
    Then the detail page opens with a single static title at the top
    And the stuck row is listed with its game identifier
    And pending rows show their attempt count as "(attempt N)" suffix per Task 5 C19 hybrid
    When the operator taps the stuck row
    Then the attention action page opens scoped to that one game
    And the page shows the stored-score display for the queued game
    And the buttons are labelled "Retry this game result" and "Discard this game result" per Task 5 C20
    When the operator taps "Retry this game result"
    Then the refbox resubmits the score with force=true
    And on success, the queue entry clears and the indicator returns to green

    # Session notes:
    # @tested_deferred (requires linked event + induced 409)
    # walkthrough attempt: 2026-05-15 — could not complete without linked event

  @user_verified @tested_deferred
  Scenario: Discard confirmation requires two taps
    Given a stuck submission appears on the detail page
    And the operator has opened the attention action page for that row
    When the operator taps "Discard this game result" once
    Then the button text changes to "TAP AGAIN TO CONFIRM DISCARD" in upper-case
    And the discard does NOT fire yet
    When the operator taps the confirmation button a second time
    Then the queue entry is removed without resubmission
    And the row disappears from the detail page

    # Session notes:
    # @tested_deferred (requires stuck submission in queue)
    # walkthrough attempt: 2026-05-15 — could not complete without linked event

  @user_verified @tested_pass
  Scenario: Dormant — indicator is hidden when no event is linked
    Given no portal event is currently linked
    Then the portal indicator tile is not rendered on the time banner
    And the time banner falls back to its pre-feature layout
    When the operator ends a game and reaches the confirm-score screen
    Then no red advisory banner appears
    And no background health check produces a 404 in the log
    When the operator links a portal event via the portal-login flow
    Then the portal indicator tile reappears on the time banner

    # Session notes:
    # @tested_pass for the tile-hidden assertion (operator confirmed banner shows pre-feature layout 2026-05-15 09:38)
    # @tested_inconclusive for the "no background 404s" assertion — refbox log was full of HTML responses
    #   from dev.uwhportal.com even with no event linked; this could be a real dormant-guard bug
    #   OR a consequence of the wrong dev URL override path. Recorded as Finding From-Unit-7 #7.
    # @tested_deferred for the relink-restores-tile assertion (the link flow could not complete on dev)
    # walkthrough: 2026-05-15 09:38

  @user_verified @tested_deferred
  Scenario: Confirm-score red advisory warns when submissions are not landing
    Given a portal event is linked
    And the portal indicator dot is red
    When the operator ends a game and reaches the confirm-score screen
    Then a red advisory banner appears warning that submissions are not landing
    And the operator can still confirm the score (the submission queues for retry)
    And the queued submission appears on the detail page after confirm

    # Session notes:
    # @tested_deferred (requires linked event + red indicator state)
    # walkthrough attempt: 2026-05-15 — could not complete without linked event

  @user_verified @tested_deferred
  Scenario: Attempt-count suffix appears on pending rows
    Given a pending submission is in the queue
    And the indicator dot is yellow because the queue is non-empty but the item is not yet stuck
    When the operator taps the indicator tile
    Then the detail page lists the pending row
    And the row text ends with the suffix "(attempt N)" where N matches the number of retry attempts so far
    And the per-second retry timer "next retry in {N}s" does NOT appear (per Task 5 C19 hybrid decision)

    # Session notes:
    # @tested_deferred (requires pending queued item — needs network failure induction or linked event)
    # walkthrough attempt: 2026-05-15 — could not complete without linked event
    # Note: the underlying data-model + view-builder change for Task 6 Fix 2 (commit 38482fd)
    # is unit-tested via the existing portal_manager test suite (122 tests pass on audit branch tip).

  @user_verified @tested_deferred
  Scenario: Compact UWH Portal logo appears above the indicator dot
    Given a portal event is linked
    Then the portal indicator tile shows the UWH Portal compact logo above the status dot
    And the tile background is lighter than the surrounding banner so the tile reads as tappable
    And in UWR mode, a UWR logo appears in place of the UWH Portal logo
    # Note: UWR-mode portal data routing is half-finished — per Task 5 C22, the data still flows
    # to the UWH Portal API. Operator-confirmed as known half-finished state pending follow-up
    # branch `feat/refbox/uwr-portal-support`.

    # Session notes:
    # @tested_deferred (requires linked event to see the tile + logo)
    # walkthrough attempt: 2026-05-15 — could not complete without linked event
