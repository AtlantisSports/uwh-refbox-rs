Feature: Manual Alarm Button
  # The manual alarm button lets the referee operator trigger the buzzer
  # sound manually during a game. It is an opt-in feature — disabled by
  # default so operators who don't need it see no change.
  #
  # When enabled, the button appears on the main game screen. The spacebar
  # provides the same function as the button. A label on the button itself
  # tells operators about the spacebar shortcut.
  #
  # The button requires a minimum hold to fire, to prevent accidental taps:
  # - during active play (clock running, no timeout): 150ms minimum
  # - during any other state (break periods or timeouts): 1 second minimum
  # While held past the minimum, the alarm continues sounding. On release,
  # the current tone finishes its natural cycle and then stops.

  Background:
    Given the refbox sound settings

  # --- Settings page ---------------------------------------------------------

  Scenario: Alarm button is hidden when the feature is disabled
    Given the manual alarm is turned off in sound settings
    When the main game screen is displayed
    Then the alarm button is not shown

  Scenario: Alarm button is visible when the feature is enabled
    Given the manual alarm is turned on in sound settings
    When the main game screen is displayed
    Then the alarm button is shown

  Scenario: Alarm toggle is greyed out when sound is disabled
    Given sound is turned off in sound settings
    When the sound settings page is displayed
    Then the "Alarm Button" toggle is non-interactive

  Scenario: Alarm toggle defaults to off
    Given a fresh install with no saved sound settings
    When the sound settings page is displayed
    Then the "Alarm Button" toggle is off

  Scenario: Existing settings files default to manual alarm disabled
    Given a sound settings file that was saved before the manual alarm feature existed
    When the settings file is loaded
    Then the manual alarm is treated as turned off
    And no error occurs during loading

  # --- Main screen layout ----------------------------------------------------

  Scenario: Enabling the alarm button replaces the game info area
    Given the manual alarm is turned on in sound settings
    When the main game screen is displayed
    Then a compact "GAME INFO" button is shown where the game info area used to be

  Scenario: Tapping the GAME INFO button opens the game details screen
    Given the manual alarm is turned on in sound settings
    When the GAME INFO button is pressed
    Then the game details screen is shown

  Scenario: With fouls and warnings tracking on, alarm and warnings share the lower area
    Given the manual alarm is turned on in sound settings
    And fouls and warnings tracking is on
    When the main game screen is displayed
    Then the alarm button is shown on the left side of the lower area
    And the warnings panel is shown on the right side of the lower area

  Scenario: With fouls and warnings tracking off, alarm button fills the lower area
    Given the manual alarm is turned on in sound settings
    And fouls and warnings tracking is off
    When the main game screen is displayed
    Then the alarm button fills the full width of the lower area

  Scenario: Disabling the alarm button restores the original main screen
    Given the manual alarm was turned on in sound settings
    When the manual alarm is turned off in sound settings
    And the main game screen is displayed
    Then the original game info area is shown
    And the alarm button is not shown

  # --- Alarm button appearance ----------------------------------------------

  Scenario: Alarm button during active play is red with the tap prompt
    Given the manual alarm is turned on in sound settings
    And the game is in an active play period (FirstHalf, SecondHalf, OvertimeFirstHalf, OvertimeSecondHalf, or SuddenDeath) with no timeout
    When the main game screen is displayed
    Then the alarm button is shown in the red colour scheme
    And the alarm button displays "Alarm"
    And the alarm button displays "Or press Spacebar"

  Scenario: Alarm button outside active play is blue with the hold prompt
    Given the manual alarm is turned on in sound settings
    And either the game is in a break period (BetweenGames, HalfTime, PreOvertime, OvertimeHalfTime, or PreSuddenDeath) or a timeout is active
    When the main game screen is displayed
    Then the alarm button is shown in the blue colour scheme
    And the alarm button displays "Hold to Test"
    And the alarm button displays "Or hold Spacebar"

  # --- Firing the alarm (button) ---------------------------------------------

  Scenario: Holding the alarm button during active play fires the alarm after 150ms
    Given the manual alarm is turned on in sound settings
    And the game is in an active play period (FirstHalf, SecondHalf, OvertimeFirstHalf, OvertimeSecondHalf, or SuddenDeath) with no timeout
    When the alarm button is pressed and held for 150ms
    Then the alarm sound plays
    And the alarm sound continues playing while the button is held

  Scenario: Holding the alarm button during a break period or timeout fires the alarm after 1 second
    Given the manual alarm is turned on in sound settings
    And either the game is in a break period (BetweenGames, HalfTime, PreOvertime, OvertimeHalfTime, or PreSuddenDeath) or a timeout is active
    When the alarm button is pressed and held for 1 second
    Then the alarm sound plays
    And the alarm sound continues playing while the button is held

  # --- Firing the alarm (spacebar) -------------------------------------------

  Scenario: Holding the spacebar during active play fires the alarm after 150ms
    Given the manual alarm is turned on in sound settings
    And the game is in an active play period (FirstHalf, SecondHalf, OvertimeFirstHalf, OvertimeSecondHalf, or SuddenDeath) with no timeout
    When the spacebar is pressed and held for 150ms
    Then the alarm sound plays
    And the alarm sound continues playing while the spacebar is held

  Scenario: Holding the spacebar during a break period or timeout fires the alarm after 1 second
    Given the manual alarm is turned on in sound settings
    And either the game is in a break period (BetweenGames, HalfTime, PreOvertime, OvertimeHalfTime, or PreSuddenDeath) or a timeout is active
    When the spacebar is held for 1 second
    Then the alarm sound plays

  Scenario: Spacebar has no effect on non-main screens
    Given the manual alarm is turned on in sound settings
    And the operator is on the configuration, penalties, or score edit screen
    When the spacebar is pressed
    Then the alarm sound does not play

  # --- Cancelling a press before the minimum hold ----------------------------

  Scenario: Releasing the alarm button before 150ms during active play cancels the press
    Given the manual alarm is turned on in sound settings
    And the game is in an active play period (FirstHalf, SecondHalf, OvertimeFirstHalf, OvertimeSecondHalf, or SuddenDeath) with no timeout
    When the alarm button is pressed and released after less than 150ms
    Then the alarm sound does not play

  Scenario: Releasing the alarm button before 1 second during a break period or timeout cancels the press
    Given the manual alarm is turned on in sound settings
    And either the game is in a break period (BetweenGames, HalfTime, PreOvertime, OvertimeHalfTime, or PreSuddenDeath) or a timeout is active
    When the alarm button is pressed and released after less than 1 second
    Then the alarm sound does not play

  # --- Release behaviour while the alarm is playing --------------------------

  Scenario: Releasing the alarm button while the alarm is playing lets the current tone finish
    Given the manual alarm is turned on in sound settings
    And the alarm sound is currently playing because the alarm button is held
    When the alarm button is released
    Then no further alarm tones are queued
    And the currently playing tone finishes its natural cycle
    And the alarm sound stops
