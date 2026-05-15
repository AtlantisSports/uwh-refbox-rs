Feature: Language selection page
  # Unit 8 audit — Feature 1 of 4 covering the operator-facing UI chrome introduced
  # by commits 848138c (grid-selection page) and ea151ac (Turkish + UNVERIFIED).
  # Seeded 2026-05-15 from AUDIT-PLAN.md Unit 8 catalog Step 5 batched review.
  #
  # Walkthrough session 1 — 2026-05-15 — native refbox in worktree
  #   (cd .worktrees/audit-unit-8-language-ui-chrome && WAYLAND_DISPLAY= RUST_LOG=info cargo run -p refbox)
  # All scenarios @tested_pass except S8.1.6 (first launch from no config) which
  # is @tested_inconclusive — would require deleting ~/.config/refbox/*.toml
  # before launch; deferred per audit design spec §8 risk #5.

  # S8.1.1
  @user_verified @tested_pass
  Scenario: Language selection grid shows all 14 languages in romanized alphabetical order
    Given the operator has the App Options settings page open
    When the operator taps the language button in the App Options page
    Then the Language selection page opens
    And row 1 shows BAHASA INDONESIA, BAHASA MELAYU, DEUTSCH, ENGLISH (left to right)
    And row 2 shows ESPAÑOL, FILIPINO, FRANÇAIS, 한국어 (left to right)
    And row 3 shows ITALIANO, NEDERLANDS, 日本語, PORTUGUÊS (left to right)
    And row 4 shows ภาษาไทย, TÜRKÇE, 中文, and one empty slot (left to right)
    # walkthrough: 2026-05-15 — operator confirmed full grid layout

  # S8.1.2
  @user_verified @tested_pass
  Scenario: Language button in App Options opens the Language selection page
    Given the operator is on the App Options settings page
    When the operator taps the language button
    Then the Language selection page appears
    And the operator is NOT taken to any other settings page
    # walkthrough: 2026-05-15 — operator confirmed navigation

  # S8.1.3
  @user_verified @tested_pass
  Scenario: Current language is pre-selected (blue) when Language page opens
    Given the operator has previously selected DEUTSCH and tapped Done
    When the operator opens the Language selection page again
    Then the DEUTSCH button is highlighted blue
    And all other language buttons are light gray
    # walkthrough: 2026-05-15 — verified blue highlight on currently-active language across multiple state transitions (English, German, Korean, Japanese)

  # S8.1.4
  @user_verified @tested_pass
  Scenario: Tapping a language button previews the selection without changing the app
    Given the Language selection page is open
    And ENGLISH is currently highlighted blue
    When the operator taps the ITALIANO button
    Then the ITALIANO button turns blue
    And the ENGLISH button returns to light gray
    And the rest of the app UI still shows English text (not Italian)
    # walkthrough: 2026-05-15 — operator confirmed preview-only semantics

  # S8.1.5
  @user_verified @tested_pass
  Scenario: Cancel returns to App Options without changing the active language
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped DEUTSCH (DEUTSCH is highlighted blue)
    When the operator taps the Cancel button (red, bottom-left)
    Then the App Options page appears
    And the app is still running in ENGLISH
    And no language change has been applied
    # walkthrough: 2026-05-15 — operator confirmed Cancel reverts

  # S8.1.6
  @user_verified @tested_inconclusive
  Scenario: First launch with no saved language pre-selects English
    Given no language has ever been saved in the app config
    When the operator opens the Language selection page
    Then the ENGLISH button is highlighted blue
    And all other language buttons are light gray
    # walkthrough: 2026-05-15 — deferred. Worktree's ~/.config/refbox/default-config.toml had language = "English" from a prior session; the test would require deleting the config file and relaunching, which was out of scope for this walkthrough. Recorded under audit-design-spec §8 risk #5 "What was not verified."

  # S8.1.7
  @user_verified @tested_pass
  Scenario: Chosen language persists after app restart
    Given the operator selects FRANÇAIS and taps Done on the Language selection page
    And the app UI updates to French
    When the operator closes the app and opens it again
    Then the app opens in French (FRANÇAIS)
    And the Language selection page shows FRANÇAIS pre-selected
    # walkthrough: 2026-05-15 — implicitly verified by both restart trips. After English → 한국어 restart, fresh exe came up in Korean with 한국어 pre-selected. After 한국어 → English restart, fresh exe came up in English with ENGLISH pre-selected. Same persistence mechanism as the original scenario; tested with CJK family instead of French.

  # S8.1.8
  @user_verified @tested_pass
  Scenario: Action-bar buttons render text in the target language's script font
    Given the app is currently running in 한국어 (CJK font as default)
    And the Language selection page is open with 한국어 pre-selected
    When the operator taps the ENGLISH button
    Then the Cancel button shows "CANCEL" in readable Latin text (not tofu boxes)
    And the Done button shows "DONE" in readable Latin text (not tofu boxes)
    # walkthrough: 2026-05-15 — operator confirmed the ea151ac tofu fix works. Cancel/Restart action buttons rendered cleanly in Latin (Roboto) under CJK default font.
