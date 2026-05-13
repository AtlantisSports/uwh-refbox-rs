# Audit Unit 5 — Referee names display
# Seeded from AUDIT-PLAN.md on 2026-05-13 (Task 6.3).
# Test status (@tested_pass / @tested_fail / @tested_inconclusive) applied
# during Task 6.4 operator-driven UI walkthrough.

Feature: Referee names in game info

  @user_verified
  Scenario: Real referee names appear when the portal returns a name map
    Given the refbox has fetched a schedule for an event with individual referee assignments
    And the portal's /referees endpoint returned display names for every assigned user_id
    When the operator navigates to the game-info page for a game with referees
    Then the page shows the per-role grid (Chief, Timer, Water 1, Water 2, Water 3)
    And each role displays the referee's resolved display_name
    And no role shows the localized "Unknown" placeholder

  @user_verified
  Scenario: "-" placeholder appears when a referee has no display name
    Given the refbox has fetched a schedule with individual referee assignments
    And the portal's /referees endpoint returned a name map missing entries for one or more user_ids
    When the operator navigates to the game-info page for that game
    Then the unresolved roles display the literal '-' placeholder
    And the '-' placeholder is shown identically regardless of locale
    And the portal-assigned identifier code is never displayed in place of the name

  @user_verified
  Scenario: Silent degradation when the /referees endpoint fails
    Given the refbox has fetched a schedule with individual referee assignments
    And the portal's /referees endpoint call failed (network error, 404, malformed response)
    When the operator navigates to the game-info page for a game with referees
    Then every role displays the literal '-' placeholder
    And the schedule loads and displays successfully despite the failure
    And no error message is shown to the operator about the missing names

  @user_verified
  Scenario: Main view and game-info page agree on referee data
    Given the refbox has fetched a schedule with individual referee assignments
    And the portal's /referees endpoint returned a name map
    When the operator views the referee list on the main game screen
    And then navigates to the game-info page for the same game
    Then both views show the same referee names in the same role positions
    And both views use the same fallback chain (display_name then localized "Unknown")

  @user_verified
  Scenario: Account-profile user.name is never displayed
    Given the portal's /referees endpoint response contains a user.name field for one or more referees
    And the same user has a rosterName or username distinct from user.name
    When the refbox builds the name map and renders the referee list
    Then the displayed name is the rosterName (preferred) or username (fallback), never user.name
    And user.name does not appear anywhere in the refbox UI
