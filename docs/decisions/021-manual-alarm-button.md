# ADR 021: Manual Alarm Button (Retroactive)

**Status:** Accepted (retroactive)
**Date:** 2026-05-13
**Audit unit:** 4 — Manual alarm button
**Audit PR:** none until Final Integration (branch holds locally per AUDIT-PLAN.md Step 8)

> **ADR numbering note:** numbered against the expected post-merge state per Unit 1 refinement #8. Audit branch was cut from `origin/master` which only carries ADRs 001–005; ADRs 006–020 live on `docs/workspace/backlog-adrs` and the Unit 1/2/3 audit branches. The gap closes at Final Integration.

## Context

The manual alarm button feature was added between 2026-04-14 and 2026-04-17 with AI assistance across 16 commits (`bc66e1e^..ff6018b`). The feature gives the referee operator an on-screen button (and a spacebar shortcut) that triggers the buzzer manually during a game. It is an opt-in feature, off by default, and the button's behaviour adapts to the current game state — short tap during active play, hold-to-test during break periods and timeouts.

The work was done against a pre-implementation spec at `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md`. Partway through implementation, an operator review revealed the original design was wrong — the spec was rewritten in commit `9cef2c4` to switch from a "mixed model" (tap during active play; hold elsewhere; disabled in other states) to a "uniform hold model" (hold everywhere; 150ms in active play, 1s elsewhere; never greyed out), and the code was then updated to match. A companion delta plan at `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md` captured the alignment work.

The audit (2026-05-13) confirmed that the shipped behaviour matches the canonical spec, surfaced one real bug (spacebar non-main-screen gating missing — fixed in commit `bf2f7b6` on the audit branch), and recorded several deliberate divergences-from-spec where the implementation's design choices were operator-approved.

## Decision

The kept behaviour for the manual alarm button feature, after audit:

### Settings

The feature is opt-in via the **Alarm Button** toggle on the Sound settings page. The toggle is in the left column below the existing three sound rows. It defaults to Off. It is non-interactive (greyed) whenever the master **Sound Enabled** toggle is Off. The setting persists across restarts via the standard `SoundSettings::migrate()` flow.

```gherkin
Feature: Manual alarm button

  Scenario: Settings toggle defaults to Off and enables the layout when turned on
    Given a fresh install with no saved sound settings
    When the operator opens the Sound settings page
    Then the "Alarm Button" toggle shows "Off"
    And the main game screen still shows the original game info area
    When the operator turns the "Alarm Button" toggle to "On"
    And returns to the main game screen
    Then the main game screen shows the GAME INFO button and the alarm button

  Scenario: Alarm Button toggle is greyed when Sound Enabled is Off
    Given the operator is on the Sound settings page
    And the "Sound Enabled" toggle is Off
    Then the "Alarm Button" toggle is non-interactive
    And tapping it does not change its value
```

### Main screen layout when enabled

With the feature enabled, the main game screen's centre column changes: the large game info area is replaced by a compact **GAME INFO** button (tapping it still opens the same game-details overlay), and below it sits the alarm zone. The alarm zone's layout depends on whether fouls-and-warnings tracking is on:

- Fouls-and-warnings tracking **on**: the lower area splits vertically into two equal halves — alarm button on the left, **WARNINGS** summary panel on the right (relocated from its prior position below the game info area).
- Fouls-and-warnings tracking **off**: the alarm button fills the full width of the centre-column lower area.

```gherkin
  Scenario: Lower area splits vertically when fouls-and-warnings tracking is on
    Given the manual alarm is turned on in sound settings
    And fouls-and-warnings tracking is on
    When the main game screen is displayed
    Then the alarm button is shown on the left side of the lower area
    And the warnings summary panel is shown on the right side of the lower area
    And both halves are of equal width

  Scenario: Lower area is full-width when fouls-and-warnings tracking is off
    Given the manual alarm is turned on in sound settings
    And fouls-and-warnings tracking is off
    When the main game screen is displayed
    Then the alarm button fills the full width of the lower area
    And no warnings summary panel is shown alongside the alarm button
```

When the feature is disabled, the main screen is fully restored to its prior layout — the original full-size game info area in the centre, with the warnings panel below it.

```gherkin
  Scenario: Disabling the alarm button restores the original main screen
    Given the manual alarm was turned on in sound settings
    And the main game screen shows the GAME INFO button and the alarm button
    When the operator turns the "Alarm Button" toggle to "Off" in sound settings
    And returns to the main game screen
    Then the original full-size game info area is shown
    And the alarm button is not shown
    And the spacebar no longer fires the alarm
```

### Alarm behaviour: uniform hold model

The alarm button must be held for a minimum duration before firing. The duration depends on the current game state:

- **Active play with no timeout** (First Half, Second Half, Overtime halves, Sudden Death — clock running, no timeout active): **150 ms**. A press shorter than 150 ms does nothing. The 150 ms is a debounce against accidental taps.
- **Every other state** (Between Games, Half Time, Pre-Overtime, Overtime Half Time, Pre-Sudden Death, and any timeout in a play period): **1 second**.

While held past the threshold, the alarm sounds continuously. On release, no further alarm tones are queued; the currently-playing tone finishes its natural cycle, then the alarm stops.

```gherkin
  Scenario: Mouse hold past 150ms during active play fires the alarm
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in First Half with the clock running and no timeout
    When the operator presses and holds the alarm button for 200 milliseconds
    Then the alarm sound plays
    And the alarm sound continues playing while the button is held

  Scenario: Mouse tap under 150ms during active play does not fire
    Given the manual alarm is turned on in sound settings
    And sound is enabled
    And the game is in First Half with the clock running and no timeout
    When the operator presses and releases the alarm button in under 100 milliseconds
    Then the alarm sound does not play

  Scenario Outline: Active-play parity across periods
    Given the manual alarm is turned on in sound settings
    And the game is in <period> with the clock running and no timeout
    When the operator presses and holds the alarm button for 200 milliseconds
    Then the alarm sound plays

    Examples:
      | period               |
      | Second Half          |
      | Overtime First Half  |
      | Overtime Second Half |
      | Sudden Death         |

  Scenario: Active play with timeout active uses 1-second hold
    Given the manual alarm is turned on in sound settings
    And the game is in First Half with a team timeout active
    When the operator presses and releases the alarm button after 200 milliseconds
    Then the alarm sound does not play
    When the operator presses and holds the alarm button for 1.1 seconds
    Then the alarm sound plays

  Scenario: Between Games requires 1-second hold
    Given the manual alarm is turned on in sound settings
    And the game state is Between Games
    When the operator presses and releases the alarm button after 200 milliseconds
    Then the alarm sound does not play
    When the operator presses and holds the alarm button for 1.1 seconds
    Then the alarm sound plays

  Scenario Outline: Break-period parity uses 1-second hold
    Given the manual alarm is turned on in sound settings
    And the game state is <break_period>
    When the operator presses and holds the alarm button for 1.1 seconds
    Then the alarm sound plays

    Examples:
      | break_period       |
      | Half Time          |
      | Pre-Overtime       |
      | Overtime Half Time |
      | Pre-Sudden Death   |

  Scenario: Release lets the currently playing tone finish naturally
    Given the manual alarm is turned on in sound settings
    And the alarm sound is currently playing because the alarm button is held
    When the operator releases the alarm button
    Then no further alarm tones are queued
    And the currently playing tone finishes its natural cycle
    And the alarm sound then stops
```

### Button appearance

The alarm button has two colour-and-label states, both using container styles that already exist elsewhere in the theme:

- **Active play with no timeout** (the only state where the short 150 ms tap fires): **red** colour scheme, large label "Alarm", small label "Or Press Spacebar".
- **Every other state** (the 1-second hold band): **blue** colour scheme, large label "Hold to Test", small label "Or Hold Spacebar".

While the button is being held down, the container background shifts to a "pressed" variant (`red_pressed_container` or `blue_pressed_container`) as visual confirmation of the press. The button is never greyed out while the feature is enabled — the minimum hold duration alone determines whether a press fires.

```gherkin
  Scenario: Button is red with the tap prompt during active play with no timeout
    Given the manual alarm is turned on in sound settings
    And the game is in First Half with the clock running and no timeout
    When the main game screen is displayed
    Then the alarm button is shown in the red colour scheme
    And the alarm button displays the large label "Alarm"
    And the alarm button displays the small label "Or Press Spacebar"

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

  Scenario: Pressed-state container visible while button is held
    Given the manual alarm is turned on in sound settings
    And the game is in First Half with the clock running and no timeout
    When the operator presses and holds the alarm button
    Then the alarm button container background changes to a "pressed red" colour
    When the operator releases the alarm button
    Then the alarm button container background returns to red
```

### Spacebar shortcut

The spacebar mirrors the mouse press behaviour exactly — same hold-duration bands, same release semantics. The spacebar shortcut is active only when the main game screen is showing; on every other screen (configuration pages, penalty editor, score editor, etc.) the spacebar has no effect on the alarm.

```gherkin
  Scenario: Spacebar parity with mouse during active play
    Given the manual alarm is turned on in sound settings
    And the game is in First Half with the clock running and no timeout
    When the operator holds the spacebar for 200 milliseconds
    Then the alarm sound plays
    And the alarm sound continues playing while the spacebar is held

  Scenario Outline: Spacebar has no effect on non-main screens
    Given the manual alarm is turned on in sound settings
    And the operator is on the <non_main_screen>
    When the operator presses and holds the spacebar for 1.5 seconds
    Then the alarm sound does not play

    Examples:
      | non_main_screen     |
      | Sound settings page |
      | Penalty edit screen |
      | Score edit screen   |
```

### Backend behaviour (no operator-facing scenario)

- **State design:** the implementation tracks three pieces of state (`mouse_alarm_held: bool`, `spacebar_held: bool`, `alarm_delay_token: u64`) so that simultaneous mouse-and-spacebar holds are handled correctly — releasing one input while the other is still held does not stop the alarm. This is a deliberate departure from the spec's single `alarm_hold_generation: u32` counter; see "Notes on divergence" below.
- **Message variants:** five new messages are introduced (`AlarmPressed`, `AlarmReleased`, `SpacebarPressed`, `SpacebarReleased`, `AlarmDelayElapsed(u64)`), plus `BoolGameParameter::ManualAlarmEnabled` for the settings toggle. The split between mouse and spacebar messages supports the simultaneous-press case.
- **Hold-duration helper:** the match block that selects 150 ms or 1 s is extracted into a private `manual_alarm_hold_duration()` method on `RefBoxApp`, reducing drift risk between the mouse and spacebar press handlers.
- **Migration:** the new `manual_alarm_enabled: bool` field on `SoundSettings` is read from old config files via the standard pattern in `SoundSettings::migrate()`. Old configs without the field load with the default Off.
- **Sound dispatch:** the manual alarm uses a dedicated `SoundId::ManualAlarm` arm in the sound controller, structurally parallel to the existing wired-button continuous-sound path. The arm pushes the alarm to the front of the sound queue so the manual alarm immediately overrides any in-flight auto-buzzer.
- **Global mouse-release subscription:** an iced event-listen subscription emits `AlarmReleased` on every left-mouse-button release in the window. This catches the case where the operator presses the alarm and then drags the cursor off the button before releasing — the alarm still stops cleanly. The handler no-ops when no alarm is held.
- **OS key-repeat guard:** the `spacebar_held` flag short-circuits the handler when the OS sends repeat key-down events while the spacebar is held, so the hold-timer is not re-scheduled.
- **Translation keys:** six new fluent keys (`alarm-button`, `alarm`, `or-press-spacebar`, `hold-to-test`, `or-hold-spacebar`, `game-info`). At the time the feature merged, only en-US, es, and fr received the keys; the other ~10 languages were back-filled by out-of-range commits during Unit 8's language work. At the audit branch tip, all 15 languages contain all 6 keys.

### Test status (backend)

- **Migration test** (commit `38799da`): passes (`cargo test -p refbox migrate`).
- **Helper method, message routing, BoolGameParameter wiring:** covered by the type system and the configuration-page tests; verified by the full `just check` pass on the audit branch.
- **Token-cancellation logic and mouse/spacebar independence:** verified by manual test sessions 1, 2, and 3 against a running refbox (logs show correct delay-token sequencing, rapid taps under threshold yielding no fire, and the simultaneous-input case behaving per the design).

## Consequences

**Enables:**
- The referee operator can manually trigger the buzzer from the refbox without depending on the wireless remote or wired button.
- The 150 ms active-play debounce protects against accidental taps while the clock is running, when an unintended buzzer would interrupt play most severely.
- The hold-to-test pattern in break periods lets the operator confirm the buzzer is working without needing access to the wireless remote.
- The keyboard shortcut (spacebar) gives a large, fast trigger surface for poolside operators under time pressure, while remaining inert on non-main screens so text inputs are unaffected.

**Commits to maintaining:**
- The uniform-hold model. Operators rely on the same hold-to-fire mental model across every state, with only the threshold varying (150 ms vs 1 s).
- The colour/label state predicate (`is_active_play && no_timeout` → red+"Alarm", everything else → blue+"Hold to Test"). Changing the predicate would change which state shows which colour, which is operator-observable.
- The main-screen layout reshuffle when the feature is enabled (GAME INFO compact button + alarm zone replacing the large info area). Any future change to the main-screen layout has to consider both the enabled and disabled branches.

**Constrains future changes:**
- The proposed ADR 006 successor (multi-remote alarm tiles) will need to preserve the operator-observable behaviour documented above, or document explicit deltas. The audit's findings should feed the ADR 006 revision pass.
- The companion delta plan at `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md` is stale on the hold-duration constant (says 250 ms in three places; actual code and current spec agree at 150 ms). Future readers should rely on the spec, not the companion plan, for the canonical value.

## What was removed during audit

No behaviour was deleted. All 36 catalog entries were `@user_verified`. One real bug was fixed during the audit:

- **B4.11 — Spacebar firing on non-main screens.** The implementation's spacebar subscription captured key events globally and the `SpacebarPressed` handler did not check `app_state`. Pressing spacebar on a settings or edit page would start the alarm hold timer and fire after 1 s, in direct violation of spec line 75. The audit added a screen-check guard in the handler at [refbox/src/app/mod.rs:2247-2251](refbox/src/app/mod.rs#L2247-L2251); the matching `SpacebarReleased` handler did not need a guard because it already no-ops when `spacebar_held` is false. Commit `bf2f7b6`. Verified end-to-end in test session 3 (S4.16 `@tested_pass`): with the fix in place, the log shows zero "Manual alarm delay started" lines while the operator holds spacebar on the Sound Options page.

## Notes on divergence from spec

Three deliberate divergences-from-spec were `@user_verified` during the audit:

1. **State design** (B4.23) — The spec describes a single `alarm_hold_generation: u32` counter. The implementation uses three fields (`mouse_alarm_held: bool` + `spacebar_held: bool` + `alarm_delay_token: u64`). The added complexity supports a real operator use-case the spec did not anticipate: holding mouse and spacebar at the same time, then releasing one input while the other is still held — the alarm continues to sound. Operator-approved; code design wins.

2. **State naming and type-width** (B4.24, B4.25) — The spec calls the counter `alarm_hold_generation: u32`; the code uses `alarm_delay_token: u64`. The rename reflects the "one token per scheduled delay" semantic the code uses; the wider type prevents overflow in any practical session. Cosmetic divergences, operator-approved; code wins.

3. **Message variant count** (B4.21) — The spec describes three message variants (`AlarmPressed`, `AlarmReleased`, `AlarmFired(u32)`). The code has five (the spec's three plus `SpacebarPressed` and `SpacebarReleased`, with `AlarmFired` renamed to `AlarmDelayElapsed(u64)`). The mouse-vs-spacebar split supports the simultaneous-press design in B4.23.

One **process finding** was recorded as a divergence to learn from rather than fix:

4. **Doc-revision meta-event** (B4.32) — Commit `9cef2c4` rewrote the spec, the original plan, and the `.feature` file mid-flight to switch from the original mixed-model design to the uniform-hold model the code was then updated to match. The retroactive-spec-update is the textbook example of why this audit exists: a future reader cannot tell from the spec alone that there was an earlier, different design. The audit logged a Process refinement (Unit 4 refinement #1 in AUDIT-PLAN.md): when operator review reveals the spec was wrong, leave the original spec accurate-as-of-its-date and write a delta ADR or companion design note recording the change — do not retroactively rewrite the spec itself.

## What was not verified

- **S4.15 — "currently-playing tone finishes its natural cycle on release."** The release path was processed cleanly in every test session (log shows `Manual alarm released` then `stop_manual_buzzer` semantics). The audibility of the natural-cycle finish — that the tone is not abruptly cut, that no further cycles queue — is documented in the audio library's behaviour but was not audibly verified during this audit because the test machine has no audio output device (ALSA errors at startup). Marked `@tested_pass` with this caveat. Verify at a tournament setup with working audio if any future change touches the sound dispatch path.
- **Default-Off on a fresh install.** The migration test in commit `38799da` covers this at the code level; the audit walked the toggle on a refbox instance whose stored settings already had the field, so the literal "fresh-install default" was inferred from the migration test rather than observed end-to-end.

## References

- **Canonical spec:** `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md`
- **Audit-design spec:** `docs/superpowers/specs/2026-05-13-audit-unit-4-manual-alarm-design.md`
- **Per-unit audit plan:** `docs/superpowers/plans/2026-05-13-audit-unit-4-manual-alarm-button.md`
- **Companion delta plan (acknowledged stale on hold-duration constant):** `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md`
- **Original 815-line plan (retconned mid-flight; reference only):** `docs/superpowers/plans/2026-04-14-manual-alarm-button.md`
- **Audit scenarios:** `refbox/tests/features/manual_alarm.feature`
- **Proposed successor:** `docs/decisions/006-multi-remote-alarm-buttons.md` — multi-remote alarm tiles. Gated on the findings recorded in this ADR; the post-audit ADR backlog pass will revisit ADR 006 with this audit's input.
- **Audit branch:** `audit/refbox/manual-alarm-button` (local-only; not pushed; awaiting Final Integration)
- **Audit commits on branch:**
  - `bf2f7b6` — `fix(refbox): gate spacebar alarm on main page only (per Unit 4 audit B4.11)`
  - `72dbef8` — `docs(refbox): align manual_alarm.feature with audit unit 4 scenarios`
  - `1b7a3c4` — `docs(refbox): record test session results for manual alarm (audit unit 4)`
- **Original 16 commits (chronological, oldest first):** `bc66e1e`, `38799da`, `b3fde3e`, `40a857a`, `5685a29`, `94b1c64`, `8db9387`, `36cde0e`, `2a4294e`, `babe3ca`, `bc620ff`, `d5f485a`, `9cef2c4`, `7f173ee`, `c90348b`, `ff6018b`
