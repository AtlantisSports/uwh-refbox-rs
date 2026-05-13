# Manual alarm button — audit scenarios
#
# Source of truth: Audit Unit 4 (see AUDIT-PLAN.md, gitignored working file).
# The audit deliberately omits `Background:` blocks so each scenario is
# reviewable in isolation during the per-unit review session and the test
# session walkthrough.

@user_verified
Feature: Manual alarm button

  @user_verified
  Scenario: Mouse hold past 150ms during active play fires the alarm
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in First Half with the clock running and no timeout
    When the operator presses and holds the alarm button for 200 milliseconds
    Then the alarm sound plays
    And the alarm sound continues playing while the button is held

  @user_verified
  Scenario: Mouse tap under 150ms during active play does not fire
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in First Half with the clock running and no timeout
    When the operator presses and releases the alarm button in under 100 milliseconds
    Then the alarm sound does not play

  @user_verified
  Scenario Outline: Active-play parity across periods
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in <period> with the clock running and no timeout
    When the operator presses and holds the alarm button for 200 milliseconds
    Then the alarm sound plays

    Examples:
      | period               |
      | Second Half          |
      | Overtime First Half  |
      | Overtime Second Half |
      | Sudden Death         |

  @user_verified
  Scenario: Spacebar parity with mouse during active play
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in First Half with the clock running and no timeout
    When the operator holds the spacebar for 200 milliseconds
    Then the alarm sound plays
    And the alarm sound continues playing while the spacebar is held

  @user_verified
  Scenario: Active play with timeout active uses 1-second hold
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in First Half with a team timeout active
    When the operator presses and releases the alarm button after 200 milliseconds
    Then the alarm sound does not play
    When the operator presses and holds the alarm button for 1.1 seconds
    Then the alarm sound plays

  @user_verified
  Scenario: Between Games requires 1-second hold
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game state is Between Games
    When the operator presses and releases the alarm button after 200 milliseconds
    Then the alarm sound does not play
    When the operator presses and holds the alarm button for 1.1 seconds
    Then the alarm sound plays

  @user_verified
  Scenario Outline: Break-period parity uses 1-second hold
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game state is <break_period>
    When the operator presses and holds the alarm button for 1.1 seconds
    Then the alarm sound plays

    Examples:
      | break_period       |
      | Half Time          |
      | Pre-Overtime       |
      | Overtime Half Time |
      | Pre-Sudden Death   |

  @user_verified
  Scenario: Settings toggle defaults to Off and enables the layout when turned on
    Given a fresh install with no saved sound settings
    When the operator opens the Sound settings page
    Then the "Alarm Button" toggle shows "Off"
    And the main game screen still shows the original game info area
    When the operator turns the "Alarm Button" toggle to "On"
    And returns to the main game screen
    Then the main game screen shows the GAME INFO button and the alarm button

  @user_verified
  Scenario: Alarm Button toggle is greyed when Sound Enabled is Off
    Given the operator is on the Sound settings page
    And the "Sound Enabled" toggle is Off
    Then the "Alarm Button" toggle is non-interactive
    And tapping it does not change its value

  @user_verified
  Scenario: Lower area splits vertically when fouls-and-warnings tracking is on
    Given the manual alarm is turned on in sound settings
    And fouls-and-warnings tracking is on
    When the main game screen is displayed
    Then the alarm button is shown on the left side of the lower area
    And the warnings summary panel is shown on the right side of the lower area
    And both halves are of equal width

  @user_verified
  Scenario: Lower area is full-width when fouls-and-warnings tracking is off
    Given the manual alarm is turned on in sound settings
    And fouls-and-warnings tracking is off
    When the main game screen is displayed
    Then the alarm button fills the full width of the lower area
    And no warnings summary panel is shown alongside the alarm button

  @user_verified
  Scenario: Button is red with the tap prompt during active play with no timeout
    Given the manual alarm is turned on in sound settings
    And the game is in First Half with the clock running and no timeout
    When the main game screen is displayed
    Then the alarm button is shown in the red colour scheme
    And the alarm button displays the large label "Alarm"
    And the alarm button displays the small label "Or Press Spacebar"

  @user_verified
  Scenario Outline: Button is blue with the hold prompt outside active-play-no-timeout
    Given the manual alarm is turned on in sound settings
    And the game state is <state>
    When the main game screen is displayed
    Then the alarm button is shown in the blue colour scheme
    And the alarm button displays the large label "Hold to Test"
    And the alarm button displays the small label "Or Hold Spacebar"

    Examples:
      | state                                  |
      | Between Games                          |
      | Half Time                              |
      | Pre-Overtime                           |
      | Overtime Half Time                     |
      | Pre-Sudden Death                       |
      | First Half with a team timeout active  |

  @user_verified
  Scenario: Pressed-state container visible while button is held
    Given the manual alarm is turned on in sound settings
    And the game is in First Half with the clock running and no timeout
    When the operator presses and holds the alarm button
    Then the alarm button container background changes to a "pressed red" colour
    When the operator releases the alarm button
    Then the alarm button container background returns to red

  @user_verified
  Scenario: Release lets the currently playing tone finish naturally
    Given the manual alarm is turned on in sound settings
    And the alarm sound is currently playing because the alarm button is held
    When the operator releases the alarm button
    Then no further alarm tones are queued
    And the currently playing tone finishes its natural cycle
    And the alarm sound then stops

  @user_verified
  Scenario Outline: Spacebar has no effect on non-main screens
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the operator is on the <non_main_screen>
    When the operator presses and holds the spacebar for 1.5 seconds
    Then the alarm sound does not play

    Examples:
      | non_main_screen     |
      | Sound settings page |
      | Penalty edit screen |
      | Score edit screen   |

  @user_verified
  Scenario: Disabling the alarm button restores the original main screen
    Given the manual alarm was turned on in sound settings
    And the main game screen shows the GAME INFO button and the alarm button
    When the operator turns the "Alarm Button" toggle to "Off" in sound settings
    And returns to the main game screen
    Then the original full-size game info area is shown
    And the alarm button is not shown
    And the spacebar no longer fires the alarm
