# Audit Unit 5 — Referee names display
# Seeded from AUDIT-PLAN.md on 2026-05-13 (Task 6.3).
# Test status (@tested_pass / @tested_fail / @tested_inconclusive) applied
# during Task 6.4 operator-driven UI walkthrough.
#
# Session 1 — 2026-05-14, against production portal (api.uwhportal.com),
# event ca-2026-canadian-underwater-hockey-nationals (Next Game: 1, RFISH vs Cornwall).
# Game-info page displayed: Chief Ref: Lewis Saleem, Timer: -, Water Ref 1: Ayman,
# Water Ref 2: Darryl Brambilla, Water Ref 3: John Kulsa. Main view showed identical
# names. PII verified via curl on /referees endpoint: Lewis Saleem's rosterName
# differs from user.name (Lewis Adam Saleem); displayed name is the rosterName,
# user.name never surfaced.

Feature: Referee names in game info

  @user_verified @tested_pass
  Scenario: Real referee names appear when the portal returns a name map
    Given the refbox has fetched a schedule for an event with individual referee assignments
    And the portal's /referees endpoint returned display names for every assigned user_id
    When the operator navigates to the game-info page for a game with referees
    Then the page shows the per-role grid (Chief, Timer, Water 1, Water 2, Water 3)
    And each role displays the referee's resolved display_name
    And no role shows the '-' placeholder for a slot that has a resolved name

  @user_verified @tested_pass
  Scenario: "-" placeholder appears when a referee has no display name
    Given the refbox has fetched a schedule with individual referee assignments
    And the portal's /referees endpoint returned a name map missing entries for one or more user_ids
    When the operator navigates to the game-info page for that game
    Then the unresolved roles display the literal '-' placeholder
    And the '-' placeholder is shown identically regardless of locale
    And the portal-assigned identifier code is never displayed in place of the name

  @user_verified @tested_inconclusive
  Scenario: Silent degradation when the /referees endpoint fails
    Given the refbox has fetched a schedule with individual referee assignments
    And the portal's /referees endpoint call failed (network error, 404, malformed response)
    When the operator navigates to the game-info page for a game with referees
    Then every role displays the literal '-' placeholder
    And the schedule loads and displays successfully despite the failure
    And a warn-level log line records the fetch failure
    And no error message is shown to the operator about the missing names

  # @tested_inconclusive (2026-05-14, Session 1): /referees endpoint succeeded
  # reliably against production throughout the session, so the silent-failure
  # path was not exercised. Code-level verification: the warn! log and
  # silent-degradation match expression were applied in Task 5 Commit 5
  # (refbox/src/app/mod.rs). ADR 022's "What was not verified" section
  # captures this gap.

  @user_verified @tested_pass
  Scenario: Main view and game-info page agree on referee data
    Given the refbox has fetched a schedule with individual referee assignments
    And the portal's /referees endpoint returned a name map
    When the operator views the referee list on the main game screen
    And then navigates to the game-info page for the same game
    Then both views show the same referee names in the same role positions
    And both views use the same fallback chain (display_name then '-')

  @user_verified @tested_pass
  Scenario: Account-profile user.name is never displayed
    Given the portal's /referees endpoint response contains a user.name field for one or more referees
    And the same user has a rosterName or username distinct from user.name
    When the refbox builds the name map and renders the referee list
    Then the displayed name is the rosterName (preferred) or username (fallback), never user.name
    And user.name does not appear anywhere in the refbox UI
