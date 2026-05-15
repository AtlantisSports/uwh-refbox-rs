Feature: UNVERIFIED marker on language buttons
  # Unit 8 audit — Feature 3 of 4. Every language button except English,
  # Spanish, and French shows a small "(UNVERIFIED)"-equivalent note in its
  # language's own script beneath the language name. The note is hardcoded
  # per-language at the call site (NOT routed through fl!) because fl!
  # renders in the current locale, defeating the purpose of a per-language
  # self-labeling button.
  #
  # Walkthrough session 1 — 2026-05-15 — see language-selection-page.feature

  # S8.3.1
  @user_verified @tested_pass
  Scenario: TÜRKÇE button appears in row 4 column 2 of the language grid
    Given the operator opens the Language selection page
    Then the TÜRKÇE button is visible in row 4, column 2 (between ภาษาไทย and 中文)
    # walkthrough: 2026-05-15 — operator confirmed TÜRKÇE position

  # S8.3.2
  @user_verified @tested_pass
  Scenario: Language buttons for unverified translations show a small note in the button
    Given the operator opens the Language selection page
    Then the TÜRKÇE button shows "(DOĞRULANMAMIŞ)" in small text below "TÜRKÇE"
    And the 中文 button shows "(未验证)" in small text below "中文"
    And the 한국어 button shows "(검증되지 않음)" in small text below "한국어"
    And the DEUTSCH button shows "(NICHT VERIFIZIERT)" in small text below "DEUTSCH"
    # walkthrough: 2026-05-15 — operator confirmed all four UNVERIFIED notes in correct scripts

  # S8.3.3
  @user_verified @tested_pass
  Scenario: ENGLISH button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ENGLISH button shows only "ENGLISH" with no note below it
    # walkthrough: 2026-05-15 — confirmed ENGLISH exemption

  # S8.3.4
  @user_verified @tested_pass
  Scenario: ESPAÑOL button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the ESPAÑOL button shows only "ESPAÑOL" with no note below it
    # walkthrough: 2026-05-15 — confirmed ESPAÑOL exemption

  # S8.3.5
  @user_verified @tested_pass
  Scenario: FRANÇAIS button shows no UNVERIFIED note
    Given the operator opens the Language selection page
    Then the FRANÇAIS button shows only "FRANÇAIS" with no note below it
    # walkthrough: 2026-05-15 — confirmed FRANÇAIS exemption

  # S8.3.6
  @user_verified @tested_pass
  Scenario: Bahasa Indonesia and Bahasa Melayu buttons show name as one small-text line plus note
    Given the operator opens the Language selection page
    Then the Bahasa Indonesia button shows "BAHASA INDONESIA" as a single smaller-text line
    And below it shows "(BELUM DIVERIFIKASI)" in small text
    And the Bahasa Melayu button shows "BAHASA MELAYU" as a single smaller-text line
    And below it shows "(BELUM DISAHKAN)" in small text
    # walkthrough: 2026-05-15 — operator confirmed the ea151ac 2-line shape works. Promoted from @proposed (B8.22 walkthrough-deferred) to @user_verified during Task 7 Step 2. B8.22 catalog entry marked @user_verified accordingly.
