Feature: Button-text damage-tracking workaround
  # Unit 8 audit — Feature 4 of 4. Every button helper in the app
  # (make_button, make_smaller_button, make_small_button, make_multi_label_button,
  # and the period-text container inside make_game_time_button) was rewritten to
  # wrap a width(Shrink) text widget inside a centering container. This is the
  # iced-0.13 damage-tracking workaround for old glyph pixels bleeding through
  # when text content changes script (e.g. when the operator changes language
  # and the time bar period name changes from "FIRST HALF" to its Korean,
  # Japanese, Mandarin, or Thai equivalent).
  #
  # Scenarios in this file are regression-coverage rather than new-behaviour:
  # they verify the sweep didn't break existing screens.
  #
  # Walkthrough session 1 — 2026-05-15 — see language-selection-page.feature

  # S8.4.1
  @user_verified @tested_pass
  Scenario: Period name in game-time button does not show ghost pixels from the previous name
    Given the operator is on any screen that shows the game-time button
    And the current period is displayed as "FIRST HALF"
    When the game advances to "SECOND HALF"
    Then the game-time button shows "SECOND HALF" cleanly with no remnant pixels from "FIRST HALF"
    # walkthrough: 2026-05-15 — operator confirmed no ghost pixels during multiple language-script transitions (English → German → Korean → Japanese → English). Period text rendered cleanly throughout.

  # S8.4.2
  @user_verified @tested_pass
  Scenario: Existing config pages still display button text correctly after the button helper changes
    Given the operator is on the Main Config page
    When the operator navigates through the Game Options, App Options, Display, and Sound settings pages
    Then all button labels on each page render centered and readable
    And no button shows truncated, overlapping, or bleeding text
    # walkthrough: 2026-05-15 — operator swept Main / Game Options / App Options / Display / Sound config pages; no regressions found

  # S8.4.3
  @user_verified @tested_pass
  Scenario: Two-line buttons on existing screens still render with both lines centered
    Given the operator navigates to any config page that shows a two-line button
    When the operator views the button
    Then both lines of text appear centered within the button
    And neither line is clipped or misaligned
    # walkthrough: 2026-05-15 — operator confirmed multi-label buttons render correctly elsewhere in the app
