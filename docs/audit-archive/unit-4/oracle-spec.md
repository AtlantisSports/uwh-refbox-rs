# Manual Alarm Button — Design Spec

**Date:** 2026-04-14
**Scope:** `refbox` crate only
**Branch type:** `feat/refbox/`

---

## Overview

Add an on-screen alarm button and spacebar shortcut that allows the referee operator to manually
trigger the buzzer sound. The feature is opt-in via the Sound settings page and is only active
during specific game states.

---

## Main Screen Layout

When the feature is enabled, the center column of the main game screen changes:

**Before (current):**
- Game clock
- Context buttons (Start Now / Foul / Warning)
- Large game info area (team names, game number, court)

**After (with feature enabled):**
- Game clock
- Context buttons (Start Now, or Foul + Warning side-by-side) — **unchanged from today**
- Compact **"GAME INFO"** button — same gray color scheme and border as the current info area;
  tapping it opens the game details screen (same as tapping the info area today)
- Lower area filling remaining space — layout depends on fouls & warnings tracking mode:

  **Fouls & warnings tracking OFF:**
  - Single gray bordered container, full width. The **"Alarm"** button is centered inside at
    ~75% of the container width. Button label: "Alarm" (large), "Or Press Spacebar" (smaller,
    below).

  **Fouls & warnings tracking ON:**
  - The lower area splits vertically into two equal halves:
    - **Left half:** gray bordered container with the Alarm button centered inside (same
      proportions and padding as the full-width version)
    - **Right half:** the existing warnings summary panel (relocated here from its current
      position below the game info area)

When the feature is disabled, the main screen is unchanged from today.

### Alarm button appearance

The button has two visual states, both using colour schemes that already exist in the code:

- **Active play, no timeout** (tap-to-fire): **red** colour scheme. Label: "Alarm" (large),
  "Or press Spacebar" (smaller, below).
- **All other states** (break periods and timeouts; hold-to-fire): **blue** colour scheme. Label:
  "Hold to Test" (large), "Or hold Spacebar" (smaller, below).

The button is never greyed out while the feature is enabled — it is always interactive, with the
minimum hold duration determining whether a press fires the alarm.

---

## Alarm Behavior by Game State

A uniform hold model applies across all game states: the button must be held for a minimum
duration before the alarm fires. While held past that minimum, the alarm continues sounding. On
release, no further tones are queued; the currently playing tone finishes its natural cycle,
then the alarm stops.

The minimum hold duration differs by state:

| Game state | Minimum hold | Behavior |
|---|---|---|
| First Half, Second Half, Overtime halves, Sudden Death — clock running, no timeout | **150ms** | Hold button (or spacebar) for at least 150ms → alarm fires and continues while held |
| Any of the above — timeout active | **1 second** | Hold button (or spacebar) for at least 1 second → alarm fires and continues while held |
| Between Games, Half Time, Pre-Overtime, Overtime Half Time, Pre-Sudden Death | **1 second** | Hold button (or spacebar) for at least 1 second → alarm fires and continues while held |
| Any other screen (config, penalties, score edit, etc.) | — | Spacebar has no effect |

The 150ms active-play minimum is a debounce to reject stray/accidental taps. 150ms still feels
like a normal button press to an operator deliberately pressing it; it is not a "long press".
The value is a single constant and can be tuned based on real-world use.

---

## Settings Integration

A new **"Alarm Button"** toggle is added to the Sound settings page (`ConfigPage::Sound`).

**Placement:** New row below the existing three rows of sound settings, on the left column,
above the Manage Remotes section. Matches the style of existing value buttons (label + On/Off
value).

**Dependencies:**
- Greyed out (non-interactive) when Sound Enabled is Off — consistent with all other
  sound-dependent toggles on this page.
- Defaults to **Off**.

**Effect:**
- Off: main screen layout is unchanged; spacebar does nothing alarm-related.
- On (and Sound Enabled On): main screen switches to the new layout; alarm is active per the
  behavior table above.

---

## Implementation Touchpoints

### New setting
- Add `manual_alarm_enabled: bool` (default `false`) to `SoundSettings` in
  `refbox/src/sound_controller/mod.rs`
- Add migration support in `SoundSettings::migrate()`

### New messages (`refbox/src/app/message.rs`)
- `Message::AlarmPressed` — fired on button pointer-down or spacebar key-down
- `Message::AlarmReleased` — fired on button pointer-up or spacebar key-up
- `Message::AlarmFired` — the actual trigger; carries a generation counter to handle
  the case where the user releases before the 1-second hold completes

### App state (`refbox/src/app/mod.rs`)
- Track `alarm_hold_generation: u32` to cancel stale hold timers without needing task
  cancellation
- On `AlarmPressed`:
  - Determine minimum hold duration from current state: 150ms for active play with no timeout,
    1000ms for everything else (break periods or timeouts in play periods)
  - Increment generation, spawn `Task::future(sleep(duration))` returning
    `Message::AlarmFired(generation)`
  - Remember that the alarm is being held (so the release handler knows to stop the sound)
- On `AlarmFired(gen)`: only start the alarm if `gen == alarm_hold_generation` (i.e. not
  cancelled by an early release). Start the buzzer in a continuous mode so it keeps sounding
  while the button is held.
- On `AlarmReleased`:
  - Increment generation (invalidates any pending hold timer that hasn't fired yet)
  - If the alarm is currently sounding, stop queueing further tones but let the currently
    playing tone finish its natural cycle

### Keyboard subscription (`refbox/src/app/mod.rs`)
- Extend `subscription()` to also subscribe to keyboard events
- Map Space key-down → `Message::AlarmPressed`, Space key-up → `Message::AlarmReleased`
- Only active when `manual_alarm_enabled && sound_enabled`

### Main view (`refbox/src/app/view_builders/main_view.rs`)
- When `manual_alarm_enabled && sound_enabled`:
  - Replace the game info button with a stacked layout: GAME INFO button + alarm container
  - Use `mouse_area` widget to capture pointer press and release on the alarm button
    (iced's standard `button` only fires on release; press-down tracking requires `mouse_area`)
  - Determine the button's colour scheme and label from the current game state:
    - Active play with no timeout → red, "Alarm / Or press Spacebar"
    - Any other state → blue, "Hold to Test / Or hold Spacebar"

### Sound settings page (`refbox/src/app/view_builders/configuration.rs`)
- Add a new row in `make_sound_config_page` with `BoolGameParameter::ManualAlarmEnabled`
  in the left column
- Grey it out when `!sound.sound_enabled`

### New message variant (`refbox/src/app/message.rs`)
- Add `BoolGameParameter::ManualAlarmEnabled`
- Handle in `update()` to toggle `sound.manual_alarm_enabled`

### Translations (`refbox/translations/`)
- Required keys: `alarm-button` (settings label), `alarm` + `or-press-spacebar` (red-state
  labels), `hold-to-test` + `or-hold-spacebar` (blue-state labels), `game-info` (GAME INFO
  button label)
- Add to every supported language file in `refbox/translations/` (not just English/Spanish/
  French — the project currently supports ~13 languages)

---

## What Is Not Changing

- The existing automatic buzzer (fires at end of game period) — unchanged
- The wireless remote behaviour — unchanged
- The wired button behaviour — unchanged
- The Manage Remotes page — unchanged
- All other screens and settings — unchanged
