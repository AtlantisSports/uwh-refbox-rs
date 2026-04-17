Feature: Confirm-Score Timing Fix
  # Tests for the tournament manager state machine around score confirmation.
  # Bug observed 6 times at tournaments (Jan 13, Jan 19, Feb 24 2026):
  # when "confirm scores" is turned OFF, the refbox would become unresponsive
  # ~90 seconds after the second half ended, requiring a restart.

  Background:
    Given a game is configured with "confirm scores" turned off

  Scenario: Confirm-pause state is cleared when the second half ends
    Given the tournament is set up with score confirmation turned off
    And the second half is in progress
    When the second half ends
    Then the refbox moves normally to the between-games period
    And remains fully responsive
