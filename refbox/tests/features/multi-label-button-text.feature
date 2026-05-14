Feature: Multi-label button text
  The multi-label button (used for two-line button labels on state-transition
  screens) must keep its text within the button's clip bounds across every
  iced re-render triggered by a game-state change.

  @user_verified
  Scenario: Two-line button text survives a state transition
    Given the refbox is on the main game screen
    And a multi-label button is visible (e.g. the start-clock / score buttons)
    When the game state changes (e.g. start clock, end half, score confirm)
    Then both lines of button text remain fully visible
    And no character is clipped against the button's edge

    # Session notes:
    # @tested_pass — operator confirmed both lines render without clipping across state transitions
    # walkthrough: 2026-05-15 00:10
