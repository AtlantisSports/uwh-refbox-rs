# Unit 8 audit scenarios — Language UI chrome
#
# Seeded 2026-05-15 from AUDIT-PLAN.md Unit 8 Behaviour catalog (commit 848138c +
# ea151ac). All 21 scenarios below were marked @user_verified by the operator
# during Task 5 batched review on 2026-05-15. One additional scenario S8.3.6
# (Bahasa button shape) is deferred to walkthrough — see B8.22 in AUDIT-PLAN.md;
# will be appended to this file as @user_verified if the operator chooses
# "keep current 2-line shape" at Task 7 Step 7.11.
#
# Each scenario carries `@user_verified` until the walkthrough adds a
# `@tested_pass` / `@tested_fail` / `@tested_inconclusive` tag. Session notes
# (date + observations) live in a comment block below each scenario after the
# walkthrough.

Feature: Language selection page

  # S8.1.1
  @user_verified
  Scenario: Language selection grid shows all 14 languages in romanized alphabetical order
    Given the operator has the App Options settings page open
    When the operator taps the language button in the App Options page
    Then the Language selection page opens
    And row 1 shows BAHASA INDONESIA, BAHASA MELAYU, DEUTSCH, ENGLISH (left to right)
    And row 2 shows ESPAÑOL, FILIPINO, FRANÇAIS, 한국어 (left to right)
    And row 3 shows ITALIANO, NEDERLANDS, 日本語, PORTUGUÊS (left to right)
    And row 4 shows ภาษาไทย, TÜRKÇE, 中文, and one empty slot (left to right)
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.2
  @user_verified
  Scenario: Language button in App Options opens the Language selection page
    Given the operator is on the App Options settings page
    When the operator taps the language button
    Then the Language selection page appears
    And the operator is NOT taken to any other settings page
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.3
  @user_verified
  Scenario: Current language is pre-selected (blue) when Language page opens
    Given the operator has previously selected DEUTSCH and tapped Done
    When the operator opens the Language selection page again
    Then the DEUTSCH button is highlighted blue
    And all other language buttons are light gray
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.4
  @user_verified
  Scenario: Tapping a language button previews the selection without changing the app
    Given the Language selection page is open
    And ENGLISH is currently highlighted blue
    When the operator taps the ITALIANO button
    Then the ITALIANO button turns blue
    And the ENGLISH button returns to light gray
    And the rest of the app UI still shows English text (not Italian)
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.5
  @user_verified
  Scenario: Cancel returns to App Options without changing the active language
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped DEUTSCH (DEUTSCH is highlighted blue)
    When the operator taps the Cancel button (red, bottom-left)
    Then the App Options page appears
    And the app is still running in ENGLISH
    And no language change has been applied
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.6
  @user_verified
  Scenario: First launch with no saved language pre-selects English
    Given no language has ever been saved in the app config
    When the operator opens the Language selection page
    Then the ENGLISH button is highlighted blue
    And all other language buttons are light gray
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.7
  @user_verified
  Scenario: Chosen language persists after app restart
    Given the operator selects FRANÇAIS and taps Done on the Language selection page
    And the app UI updates to French
    When the operator closes the app and opens it again
    Then the app opens in French (FRANÇAIS)
    And the Language selection page shows FRANÇAIS pre-selected
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.1.8
  @user_verified
  Scenario: Action-bar buttons render text in the target language's script font
    Given the app is currently running in 한국어 (CJK font as default)
    And the Language selection page is open with 한국어 pre-selected
    When the operator taps the ENGLISH button
    Then the Cancel button shows "CANCEL" in readable Latin text (not tofu boxes)
    And the Done button shows "DONE" in readable Latin text (not tofu boxes)
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

Feature: Restart-required indicator and flow

  # S8.2.1
  @user_verified
  Scenario: Switching between two Latin-script languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps DEUTSCH
    Then the confirm button (bottom-right) shows "FERTIG" in green
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.2.2
  @user_verified
  Scenario: Switching from a Latin language to a CJK language shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps 한국어
    Then the confirm button (bottom-right) shows "재시작하여 적용" in blue
    And the Done/green button is not visible
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.2.3
  @user_verified
  Scenario: Switching between two CJK languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in 한국어
    When the operator taps 日本語
    Then the confirm button (bottom-right) shows a green Done button in Japanese ("完了")
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.2.4
  @user_verified
  Scenario: Switching from a Latin language to Thai shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps ภาษาไทย
    Then the confirm button (bottom-right) shows a blue Restart button in Thai text
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.2.5
  @user_verified
  Scenario: Tapping Restart saves the language, closes the app, and opens a fresh instance
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped 한국어 (blue Restart button is visible)
    When the operator taps the blue Restart button
    Then the app closes
    And a new instance of the app opens
    And the new instance is running in 한국어 with Korean as the UI language
    And the Language selection page shows 한국어 pre-selected if reopened
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

Feature: UNVERIFIED marker on language buttons

  # S8.3.1
  @user_verified
  Scenario: TÜRKÇE button appears in row 4 column 2 of the language grid
    Given the operator opens the Language selection page
    Then the TÜRKÇE button is visible in row 4, column 2 (between ภาษาไทย and 中文)
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.3.2
  @user_verified
  Scenario: Language buttons for unverified translations show a small note in the button
    Given the operator opens the Language selection page
    Then the TÜRKÇE button shows "(DOĞRULANMAMIŞ)" in small text below "TÜRKÇE"
    And the 中文 button shows "(未验证)" in small text below "中文"
    And the 한국어 button shows "(검증되지 않음)" in small text below "한국어"
    And the DEUTSCH button shows "(NICHT VERIFIZIERT)" in small text below "DEUTSCH"
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.3.3
  @user_verified
  Scenario: ENGLISH button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ENGLISH button shows only "ENGLISH" with no note below it
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.3.4
  @user_verified
  Scenario: ESPAÑOL button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ESPAÑOL button shows only "ESPAÑOL" with no note below it
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.3.5
  @user_verified
  Scenario: FRANÇAIS button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the FRANÇAIS button shows only "FRANÇAIS" with no note below it
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.3.6 — Bahasa button shape — deferred pending B8.22 walkthrough decision.
  # If operator confirms current 2-line small-text shape, this scenario is
  # appended here with @user_verified + the chosen test tag. If operator
  # chooses to revert to the oracle's 3-line shape, scenario stays in
  # AUDIT-PLAN.md only and a `chore/refbox/bahasa-3-line-button-shape`
  # branch is filed in Findings backlog.

Feature: Button-text damage-tracking workaround

  # S8.4.1
  @user_verified
  Scenario: Period name in game-time button does not show ghost pixels from the previous name
    Given the operator is on any screen that shows the game-time button
    And the current period is displayed as "FIRST HALF"
    When the game advances to "SECOND HALF"
    Then the game-time button shows "SECOND HALF" cleanly with no remnant pixels from "FIRST HALF"
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.4.2
  @user_verified
  Scenario: Existing config pages still display button text correctly after the button helper changes
    Given the operator is on the Main Config page
    When the operator navigates through the Game Options, App Options, Display, and Sound settings pages
    Then all button labels on each page render centered and readable
    And no button shows truncated, overlapping, or bleeding text
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  # S8.4.3
  @user_verified
  Scenario: Two-line buttons on existing screens still render with both lines centered
    Given the operator navigates to any config page that shows a two-line button
    When the operator views the button
    Then both lines of text appear centered within the button
    And neither line is clipped or misaligned
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM
