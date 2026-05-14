Feature: Timed buzzer playback
  The refbox plays a timed buzzer (~2.15 seconds + fade) when configured
  sound events fire. The buzzer must end cleanly (no click, no clipping
  distortion) and the fade-out must land in a neutral part of the waveform.

  @user_verified
  Scenario: Timed buzzer ends without an audible click
    Given the refbox is configured with a sound buzzer enabled
    When the operator triggers an event that fires a timed buzzer
    Then the buzzer plays for approximately 2.15 seconds
    And the fade-out at the end is smooth
    And no audible click or tap is heard at the moment of stop

    # Session notes (filled by Task 7):
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  @user_verified
  Scenario: Timed buzzer fade-out is not aligned with the buzzer's natural cycle
    Given the buzzer sound (Buzz / Whoop / Crazy) has a natural loop cycle
    When the timed buzzer's software fade-out runs
    Then the fade-out lands in a full-amplitude region of the buzzer's waveform
    And not at the start of a new loop cycle (which would re-attack
      as the gain ramps to zero)

    # Session notes (filled by Task 7):
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM

  @user_verified
  Scenario: Crazy buzzer body has no peak-clipping distortion
    Given the Crazy buzzer asset has been replaced (pre-fix peak amplitude 2.03)
    When the operator plays the Crazy buzzer at the default system volume
    Then no peak-clipping distortion is audible during the buzz body
    And the buzz character matches the operator's expectation of the
      Crazy sound

    # Session notes (filled by Task 7):
    # @tested_pass | @tested_fail | @tested_inconclusive
    # walkthrough: YYYY-MM-DD HH:MM
