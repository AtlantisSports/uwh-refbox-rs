@unit-3 @adr-009
Feature: Settings navigation and per-page save model
  # ADR 009 reshapes the refbox settings area. The Main settings page becomes
  # a pure 2x2 navigation grid; each editing page now carries its own
  # CANCEL/APPLY footer instead of a single global DONE button. Live side
  # effects (LED panel hide_time push, sound controller push, etc.) shift to
  # per-page commits. The full audit catalog lives in AUDIT-PLAN.md (Unit 3);
  # these scenarios capture the operator-observable behaviour the operator
  # marked @user_verified during Task 4 review.

  Background:
    Given the refbox is launched
    And the operator is on the main game screen

  # --- Main settings page ----------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.1 — Main settings shows the 2x2 grid with a single BACK button
    # Covers B3.1, B3.2, B3.3.
    When the operator taps Settings from the main game screen
    Then the Main settings page shows four equal-sized tiles arranged in two rows of two
    And the top row shows the GAME OPTIONS tile on the left and the APP OPTIONS tile on the right
    And the bottom row shows the USER OPTIONS tile on the left and the LANGUAGE tile on the right
    And no game-number picker is shown on the Main settings page
    And the footer shows a single BACK button in the left slot with the remaining slots empty
    And pressing BACK exits settings and returns to the main game screen without any prompt

  # --- User Options page -----------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.2 — User Options is a grouping page with two visible tiles and a single BACK
    # Covers B3.4, B3.5, B3.6.
    When the operator taps the USER OPTIONS tile on Main
    Then the User Options page is shown
    And the User Options page shows a three-column row with DISPLAY OPTIONS on the left, an empty middle slot, and SOUND OPTIONS on the right
    And the empty middle slot is reserved for the future View Mode button (ADR 010)
    And the footer shows a single BACK button in the left slot with the remaining slots empty
    And pressing BACK returns to the Main settings page without any prompt

  # --- Game Options chrome and Apply-disable rules ---------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.3 — Game Options shows a CANCEL / Game: N / APPLY footer
    # Covers B3.7, B3.21.
    When the operator opens the Game Options page from Main
    Then the footer's left slot shows a red CANCEL button
    And the footer's middle slot shows the game-number picker labelled "Game: N"
    And the footer's right slot shows a green APPLY button
    And no single DONE button is shown on Game Options

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.4 — APPLY on Game Options is disabled when no edits have been made
    # Covers B3.8.
    When the operator opens the Game Options page from Main
    Then the APPLY button is shown greyed out
    When the operator changes any editable field on Game Options
    Then the APPLY button becomes enabled

  @user_verified @tested_pass
  # regression-tested by 7 `uwhportal_incomplete_*` tests in commit 0b6af14; manual walk skipped 2026-05-13 (predicate exhaustively covered by unit tests; setting up portal-incomplete state manually adds no new coverage)
  Scenario: S3.5 — APPLY on Game Options stays disabled when portal state is incomplete
    # Covers B3.9. Paired with `uwhportal_incomplete_*` regression tests in commit 0b6af14.
    Given "Using UWH Portal" is turned on
    And the event, court, or schedule is missing — or the current game is not in the active schedule for the current court
    When the operator opens the Game Options page from Main
    Then the APPLY button is shown greyed out
    And changing other editable fields on Game Options does not enable APPLY
    And the APPLY button only enables once event, court, and a valid game-in-schedule are all selected

  # --- Cancel reverts page-edited fields -------------------------------------

  @user_verified @tested_pass
  # verified 2026-05-13 by operator + regression-tested by `game_snapshot_revert_*` in commit 0b6af14
  Scenario: S3.6 — CANCEL on Game Options reverts every field the operator edited on the page
    # Covers B3.10. Paired with `game_snapshot_revert_*` regression tests in commit 0b6af14.
    When the operator opens the Game Options page from Main
    And the operator changes any combination of game parameters, the event, the court, the "Using UWH Portal" toggle, or the schedule
    And the operator presses CANCEL
    Then every edited field on Game Options returns to the value it had when the page was entered
    And the operator returns to the Main settings page

  # --- Mid-game Apply confirmation -------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator — only verification path for B3.12 / B3.36 since RefBoxApp construction is not unit-testable without heavy mocking (regression-test gap; Task 6.1 limit)
  Scenario: S3.7 — Pressing APPLY on Game Options during a running game raises the Keep / End / Discard confirmation
    # Covers B3.12. The "End game and apply" choice is currently shipped landing
    # on Main settings (B3.13) — that destination is queued for a redesign and
    # is recorded in the Findings backlog; it is NOT covered by this scenario.
    Given a game is in progress (clock running, or otherwise active)
    When the operator opens the Game Options page from Main
    And the operator changes at least one editable field
    And the operator presses APPLY
    Then a confirmation page is shown with three choices: "Go back to editor", "Discard changes", and "End game and apply"
    And the same confirmation handles game-config changes, game-number-only changes, and the portal-incomplete case

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.8 — Pressing APPLY on Game Options when no game is running commits and returns to Main
    # Covers B3.11.
    Given no game is currently in progress (the clock is not running)
    When the operator opens the Game Options page from Main
    And the operator changes at least one editable field
    And the operator presses APPLY
    Then the edits are saved
    And the operator is returned to the Main settings page

  # --- Post-picker routing ---------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.9 — Picking a game from the picker returns the operator to Game Options
    # Covers B3.14. Applies to both portal mode (game list) and non-portal mode (keypad page).
    When the operator opens the Game Options page from Main
    And the operator opens the game picker (or keypad)
    And the operator picks a game (or confirms a game number)
    Then the operator returns to the Game Options page
    And the operator does NOT return to the Main settings page

  # --- Picker-driven field clearing ------------------------------------------

  @user_verified @tested_pass
  # regression-tested by `select_event_sets_event_and_clears_court_game_schedule` in commit 0b6af14; manual walk skipped 2026-05-13 (predicate covered by unit test; portal multi-event setup adds no new coverage)
  Scenario: S3.10 — Picking a new event auto-clears court, game number, and cached schedule
    # Covers B3.15. Paired with `select_event_sets_event_and_clears_*` regression test in commit 0b6af14.
    Given the operator is on Game Options in portal mode
    And an event, court, and game number are already selected
    When the operator picks a different event from the event picker
    Then the court is cleared
    And the game number is cleared
    And the cached schedule is cleared
    And the operator must re-pick court and game from the new event's filtered list
    And APPLY remains disabled until court and game-in-schedule are both re-selected

  @user_verified @tested_pass
  # regression-tested by `select_court_sets_court_and_clears_game_number` in commit 0b6af14; manual walk skipped 2026-05-13 (predicate covered by unit test; portal multi-court setup adds no new coverage)
  Scenario: S3.11 — Picking a new court auto-clears the game number
    # Covers B3.16. Paired with `select_court_sets_court_and_clears_*` regression test in commit 0b6af14.
    Given the operator is on Game Options in portal mode
    And an event, court, and game number are already selected
    When the operator picks a different court
    Then the game number is cleared
    And the operator must re-pick the game from the new court's filtered list
    And APPLY remains disabled until a valid game-in-schedule is re-selected

  # --- Game Options layout (after ce6cfeb) ----------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.12 — Game Options uses the standard 5-row layout and the new tile arrangement
    # Covers B3.18 (transitional — see TODO #1), B3.19, B3.20.
    # B3.18 note: the Single Half tile is intentionally absent until TODO #1
    # ships an entry-point inside the Half Length editor. Until then, the
    # operator has no way to toggle Single Half from the UI. This is an
    # accepted transitional state.
    When the operator opens the Game Options page from Main
    Then the page is laid out as a time bar across the top, four content rows sharing the leftover vertical space, and a fixed action row at the bottom
    And the "Using UWH Portal" toggle is shown in the left-hand column of the first content row
    And no Single Half tile is shown on Game Options
    And inter-row gaps match the other settings pages

  # --- App Options -----------------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.13 — App Options shows CANCEL/APPLY chrome, no inner Language button, and tightened rows
    # Covers B3.22, B3.23, B3.24.
    When the operator opens the App Options page from Main
    Then the footer shows a red CANCEL button on the left and a green APPLY button on the right
    And no single DONE button is shown on App Options
    And no Language button is shown inside App Options
    And the two data rows hug their content height with a single vertical spacer above the footer
    And APPLY is disabled when no App-slice fields have changed
    And APPLY becomes enabled as soon as at least one App-slice field changes
    And pressing CANCEL reverts in-flight App edits and returns to Main
    And pressing APPLY when enabled commits the App edits and returns to Main

  # --- Display Options -------------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.14 — Display Options shows CANCEL/APPLY chrome and APPLY still pushes hide_time to the LED panel
    # Covers B3.25.
    When the operator opens the Display Options page from Main
    Then the footer shows a red CANCEL button on the left and a green APPLY button on the right
    And no single DONE button is shown on Display Options
    When the operator changes the hide_time field and presses APPLY
    Then the hide_time change is saved
    And the hide_time change is pushed to the LED panel (the same side-effect that previously fired on global DONE)
    And the operator is returned to the Main settings page
    When the operator changes the hide_time field on a fresh entry and presses CANCEL
    Then the hide_time field reverts to its entry-time value
    And the operator is returned to the Main settings page

  # --- Sound Options ---------------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator (Part A chrome + sound push) and confirmed nested-cancel snapshot fix (Part B) — initial walk on 2026-05-13 surfaced a bug where Apply stayed disabled after returning from Manage Remotes; fixed in commit f9a32f4 (navigate_to_parent now re-captures the parent snapshot); regression test `sound_apply_requires_snapshot_present` in same commit.
  Scenario: S3.15 — Sound Options shows CANCEL/APPLY chrome and APPLY still pushes sound settings to the sound controller
    # Covers B3.26.
    When the operator opens the Sound Options page from Main
    Then the footer shows a red CANCEL button on the left and a green APPLY button on the right
    And no single DONE button is shown on Sound Options
    And the Manage Remotes button is shown inside Sound Options (matching the web layout)
    When the operator changes any sound setting and presses APPLY
    Then the sound settings are saved
    And the new sound settings are pushed to the sound controller
    And the operator is returned to the Main settings page
    When the operator changes a sound setting on a fresh entry and presses CANCEL
    Then the sound settings revert to their entry-time values
    And the operator is returned to the Main settings page

  # --- Manage Remotes --------------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.16 — Manage Remotes shows CANCEL/APPLY chrome with Add in the right column above the action row
    # Covers B3.27.
    When the operator opens the Manage Remotes page from Sound Options
    Then the Add button is shown in the right column above the action row
    And the bottom action row shows a red CANCEL button on the left and a green APPLY button on the right
    When the operator adds, removes, or edits remotes and presses APPLY
    Then the remotes list is saved
    And the remotes list is pushed to the sound controller
    And the operator is returned to the Sound Options page
    When the operator changes the remotes list on a fresh entry and presses CANCEL
    Then the remotes list reverts to its entry-time value
    And the operator is returned to the Sound Options page

  # --- Language page ---------------------------------------------------------

  @user_verified @manual_walkthrough_only @tested_pass
  # verified 2026-05-13 by operator
  Scenario: S3.17 — Language page shows a localized Cancel/Confirm footer aligned with the other Editing pages
    # Covers B3.28, B3.29.
    When the operator opens the Language page from Main
    Then the page shows the list of supported languages
    And the footer shows a CANCEL button on the left and a CONFIRM button on the right
    And each footer button's label is shown in the language the operator currently has selected (not the language being tapped on)
    And the footer sits at content height directly below the language grid (matching every other Editing page's footer shape)
    And the CONFIRM button is greyed out until the operator picks a language different from the current one
    And the CONFIRM button becomes enabled once a different language is selected
