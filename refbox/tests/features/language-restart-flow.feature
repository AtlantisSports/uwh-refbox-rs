Feature: Restart-required indicator and flow
  # Unit 8 audit — Feature 2 of 4. When the operator preview-selects a language
  # whose font family differs from the current app's font family (Latin / CJK /
  # Thai), the right action button changes from green DONE to blue RESTART TO
  # APPLY. Tapping RESTART saves the language to config, kills the simulator
  # child process, spawns a fresh copy of the exe, and exits.
  #
  # Walkthrough session 1 — 2026-05-15 — see language-selection-page.feature

  # S8.2.1
  @user_verified @tested_pass
  Scenario: Switching between two Latin-script languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps DEUTSCH
    Then the confirm button (bottom-right) shows "FERTIG" in green
    # walkthrough: 2026-05-15 — operator confirmed green FERTIG; hot-swap to German worked

  # S8.2.2
  @user_verified @tested_pass
  Scenario: Switching from a Latin language to a CJK language shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps 한국어
    Then the confirm button (bottom-right) shows "재시작하여 적용" in blue
    And the Done/green button is not visible
    # walkthrough: 2026-05-15 — operator confirmed blue Korean Restart button (tested from German via German → 한국어, same Latin → CJK path)

  # S8.2.3
  @user_verified @tested_pass
  Scenario: Switching between two CJK languages shows green Done button
    Given the Language selection page is open
    And the app is currently running in 한국어
    When the operator taps 日本語
    Then the confirm button (bottom-right) shows a green Done button in Japanese ("完了")
    # walkthrough: 2026-05-15 — operator confirmed green Japanese Done button after the Latin → CJK restart left the app in Korean

  # S8.2.4
  @user_verified @tested_pass
  Scenario: Switching from a Latin language to Thai shows blue Restart button
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    When the operator taps ภาษาไทย
    Then the confirm button (bottom-right) shows a blue Restart button in Thai text
    # walkthrough: 2026-05-15 — operator confirmed blue Thai Restart button; Cancel returned without restart

  # S8.2.5
  @user_verified @tested_pass
  Scenario: Tapping Restart saves the language, closes the app, and opens a fresh instance
    Given the Language selection page is open
    And the app is currently running in ENGLISH
    And the operator has tapped 한국어 (blue Restart button is visible)
    When the operator taps the blue Restart button
    Then the app closes
    And a new instance of the app opens
    And the new instance is running in 한국어 with Korean as the UI language
    And the Language selection page shows 한국어 pre-selected if reopened
    # walkthrough: 2026-05-15 — operator confirmed both restart trips: German → 한국어 (fresh exe in Korean) and 한국어 → ENGLISH (fresh exe in English). Sim child killed and respawned cleanly on each restart.
