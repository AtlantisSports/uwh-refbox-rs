Feature: Keypad player number display
  The keypad displays a player number, foul number, penalty number, or
  portal-login digit string to the right of a label. The digit string
  must render correctly regardless of length.

  @user_verified
  Scenario: Short digit string renders correctly
    Given the refbox is on a keypad page (player-number, foul, penalty,
      or portal-login)
    When the operator types one or two digits
    Then the digit string renders fully and is right-aligned in its row
    And the digit string does not vanish (the rendering bug pre-fix
      manifested as an empty render for short strings)

    # Session notes (filled by Task 7):
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  @user_verified
  Scenario: Digit text size is consistent across keypad variants
    Given the refbox is on a keypad page
    When the operator views any of the four keypad variants (player-number,
      foul, penalty, portal-login)
    Then the digit text renders at MEDIUM_TEXT size in all variants

    # Session notes (filled by Task 7):
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM
