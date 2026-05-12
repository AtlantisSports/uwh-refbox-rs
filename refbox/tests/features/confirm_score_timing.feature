Feature: Confirm-score timing fix
  # Tests for the tournament manager state machine around score confirmation.
  # Bug observed 6 times at tournaments (Jan 13, Jan 19, Feb 24 2026):
  # when "confirm scores" is turned OFF, the refbox would become unresponsive
  # ~90 seconds after the second half ended, requiring a restart.
  #
  # Audited by the AI Code Audit, Unit 1 (2026-05-12). See AUDIT-PLAN.md
  # (gitignored) for the catalog and decision log; see ADR 019 for the
  # retroactive design record.
  #
  # Test sessions:
  # (filled during Step 6.2 / 6.3 execution)

  @user_verified
  Scenario: Clock starts cleanly after the second half ends with confirm-score off
    Given the operator has "Confirm Score Required" set to OFF in Game Settings
    And a game has been configured and started
    And the second half has ended
    When the operator dismisses the score-confirmation prompt
    Then the refbox moves to the between-games period
    And the refbox remains fully responsive for at least 120 seconds afterwards
