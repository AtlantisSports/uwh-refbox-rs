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
  # 2026-05-12 - Audit Unit 1 Task 6.3 - S1.1 pass.
  #   Confirm Score Required set to OFF; half shortened to ~5s for the test
  #   (Black 0, White 1 at end of second half). Operator dismissed the
  #   score-confirmation prompt; refbox transitioned cleanly to BetweenGames,
  #   stayed responsive through the full 120-second confirm-pause window and
  #   beyond (normal "Resetting game" at +2m11s). No panic, no mutex poison.
  #   The "end_confirm_pause called while in unexpected period" warning did
  #   NOT appear, confirming B1.1 cleared the pause state cleanly before
  #   start_clock - B1.2's defensive recovery did not need to fire.

  @user_verified @tested_pass
  Scenario: Clock starts cleanly after the second half ends with confirm-score off
    Given the operator has "Confirm Score Required" set to OFF in Game Settings
    And a game has been configured and started
    And the second half has ended
    When the operator dismisses the score-confirmation prompt
    Then the refbox moves to the between-games period
    And the refbox remains fully responsive for at least 120 seconds afterwards
