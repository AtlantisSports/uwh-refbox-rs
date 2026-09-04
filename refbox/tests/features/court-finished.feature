# Written walkthrough scenarios for branch fix/refbox/court-finished-behaviour.
# Oracle: docs/superpowers/specs/2026-08-17-court-finished-behaviour-design.md, plus the
# three late decisions recorded in
# docs/superpowers/specs/2026-08-17-court-finished-behaviour-decisions.md (clock stops
# dead, the banner reads END, the 30-second whistle is suppressed).
# These files are documentation — there is no cucumber runner in this workspace.

Feature: A court whose schedule is finished
  On a multi-court event the refbox finds the upcoming game by searching its own court.
  When there is no later game on that court, the day on that court is finished: nothing
  starts by itself, nothing is adopted from another court, the table and the clock say so
  plainly, and START NOW is unavailable until the operator picks a game.

  Scenario: The last game on a court does not roll into another court's game
    Given a two-court event where court 1 holds games 1 and 3 and court 2 holds games 2 and 4
    And court 1 is selected
    When game 3 is started
    Then the refbox records that court 1's schedule is finished
    And game 4 is not adopted as the upcoming game
    And the Next Game block reads dashes, not game 4 or its team names

  Scenario: The clock stops when the last game on a court ends
    Given court 1 is selected and game 3 — the last game on court 1 — is in progress
    When game 3 ends
    Then the clock stops immediately, with no countdown toward a next game
    And the big clock block reads END over --:--
    And no buzzer, whistle or countdown beep sounds for a game that is not coming
    And game 3's own end-of-game buzzer still sounds as usual
    And game 3's final score stays on screen
    And the middle block, the settings values and the referee names all read dashes
    And no game ever starts, however long the refbox is left running
    And game 3's own result is submitted to the portal as usual

  # Covers the other ordering: a court can be marked finished while a between-games break
  # is ALREADY counting down. That countdown must expire into a stop, not into a game,
  # and must stay silent on the way there. This is the ONLY path that still reaches the
  # 30-second mark, which is why the whistle is gated as well as the buzzer and beeps.
  Scenario: A break already running when the court becomes finished expires into a stop
    Given a between-games break is counting down
    When the court's schedule becomes finished before the break expires
    Then the break reaches 0:00 and the clock holds there
    And no 30-second whistle and no start-of-play buzzer sound
    And no game starts

  Scenario: START NOW is unavailable until a game is picked
    Given court 1's schedule is finished
    Then the START NOW button is greyed and does nothing when pressed
    When the operator picks a game on court 1 in Settings
    Then START NOW is available again

  # Regression guard: the court is already "finished" from the moment the last game on it
  # STARTS, so the mid-game breaks of that very game must keep START NOW available and
  # must keep their own whistle.
  Scenario: START NOW still works at half time of the last game on a court
    Given court 1 is selected and game 3 — the last game on court 1 — is in progress
    When the first half ends and the clock reaches half time
    Then START NOW is available and starts the second half
    And the half-time 30-second whistle sounds as usual
    And the same holds for pre-overtime and pre-sudden-death breaks of that game

  # Decision 10: a finished court stays finished until the operator asks. Restarting no
  # longer loses the ability to find a late addition — the anchor survives, so the search
  # runs again on every REFRESH, in this session or any later one.
  Scenario: A game added to a finished court is adopted on REFRESH
    Given court 1's schedule is finished
    When a new game on court 1 is added in the portal
    Then nothing changes until the operator presses REFRESH
    When the operator presses REFRESH
    Then that game becomes the upcoming game
    And the clock counts down toward it again
    And START NOW is available again
    And the same holds after a restart, because the anchor is remembered

  Scenario: A restart comes back to the same finished state
    Given court 1's schedule is finished
    When the refbox is closed and reopened
    Then it returns to court 1 in the finished state within one schedule fetch
    And the remembered session file holds the court with no game number
    And that game number never becomes "1"

  # Scenario 2's Critical: with no network the old code fell back to arithmetic,
  # invented game 1, played it unattended and queued a 0-0 that was delivered on
  # reconnect.
  Scenario: A restart with no network comes back finished, not inventing a game
    Given court 1's schedule is finished
    When the refbox is closed, the network is switched off, and it is reopened
    Then it shows the finished state with no upcoming game
    And no game starts, however long it is left running
    And nothing is queued for the portal
    And nothing is posted when the network returns

  # Regression guard for the stale remembered-session note: it exists only to restore the
  # operator's place at startup, and must never fire hours later.
  Scenario: A remembered game does not resurrect itself at the end of the day
    Given the refbox was launched with a remembered game from an earlier session
    And the first schedule arrived while the operator was in Settings
    And the operator picked a different game and applied
    When that game is played and ends
    Then the end-of-game refresh does not re-adopt the remembered game
    And court 1's schedule stays finished

  # Decision 9 SUPERSEDED. A court the refbox holds no record for is either a fresh
  # morning or a replacement box brought out mid-day, and it cannot tell them apart.
  # Offering the earliest game would confidently offer a game played hours ago.
  Scenario: A court with no recorded history requires an operator pick
    Given the refbox is launched with court 1 selected and no game played on it yet
    When the schedule arrives
    Then no game is offered as the upcoming game
    And the clock is stopped and START NOW is greyed
    And no game from another court is offered
    When the operator picks a game on court 1 in Settings
    Then that game becomes the upcoming game

  # Regression guard: a refresh must not overrule the operator's own choice.
  Scenario: An out-of-order pick survives a refresh
    Given court 1 is selected and game 9 has been played
    When the operator picks game 15 in Settings and taps REFRESH
    Then game 15 remains the upcoming game, with its own teams, block and scheduled start
    And it is not replaced by game 11

  Scenario: Ordinary mid-day operation is unchanged
    Given court 1 is selected and game 1 is in progress
    Then game 3 is the upcoming game
    When game 1 ends and the break counts down to zero
    Then game 3 starts as usual

  Scenario: Manual mode is unchanged
    Given the portal is switched off
    Then games number on and auto-start exactly as before

  Scenario: Switching to manual mid-day clears the finished state
    Given court 1's schedule is finished
    When the operator switches to manual and applies
    Then the refbox no longer treats any court as finished
    And the break counts down and START NOW is available again
