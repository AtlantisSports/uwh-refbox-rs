# Design — Visual & Audible 10-Second Countdown

**Date:** 2026-06-24
**Crate:** `refbox` only (no `uwh-common`, no state machine, no wire-format structure, no wireless-remote)
**Branch (planned):** `feat/refbox/audible-countdown`

---

## Goal

1. Move the existing "Hide time for last 15 seconds" setting from the **Display Options** page to
   the **App Options** page, relabel it **"SHOW COUNTDOWN FOR LAST 10 SECONDS"**, invert its
   Yes/No, and change its threshold from 15 → 10 seconds.
2. Add a new **"AUDIBLE COUNTDOWN FOR LAST 10 SECONDS"** on/off setting next to it that plays one
   beep per second for the final 10 seconds (10→1) before each playing period begins.
3. Define a new beep sound, synthesized the same way as the prior buzzer-upgrade clips.

## Explicitly out of scope

- No change to the buzzer/whistle/start-of-period sounds for any other event.
- No countdown before non-playing periods (timeouts, between-game beyond its final 10 s before a
  half, etc.).
- No change to `uwh-common`, the tournament-manager state machine, the LED-panel/overlay wire
  format structure, or the wireless remote.

---

## Acceptance criteria (operator-observable)

- On **App Options**: a row reads **SHOW COUNTDOWN FOR LAST 10 SECONDS** (left) · **AUDIBLE
  COUNTDOWN FOR LAST 10 SECONDS** (right); the next row down has **SHOW BEHIND TIME/DELAY** (left)
  with an empty right cell.
- On **Display Options**: the moved button is gone; the **Layout** button is unchanged in place,
  with an empty cell where the moved button used to be. Nothing else moves.
- **SHOW COUNTDOWN = YES** shows the final 10-second break countdown on the scoreboard;
  **= NO** hides those last 10 seconds (shows the upcoming period's time instead). This is the
  inverse of today's "Hide" wording, and the threshold is now 10 s (was 15 s).
- **AUDIBLE COUNTDOWN = YES** plays 10 identical short beeps (one each at 10, 9, … 1 seconds
  remaining) during the break before each of: 1st Half, 2nd Half, 1st OT Half, 2nd OT Half,
  Sudden Death. Default is **NO** (off).
- The two countdowns are independent: the audible beep fires at the true seconds-remaining
  regardless of the visual Show/Hide setting.
- The existing "buzzer at period start" setting is untouched; if on, it still sounds at 0.

---

## Key architectural finding (verified)

`maybe_play_sound()` runs on the **raw** snapshot from the game clock
(`refbox/src/app/mod.rs:483`), *before* the display sender applies the hide/show mutation to its
own separate copy (`refbox/src/app/update_sender.rs:504-524`). Therefore the audible-countdown
trigger reads true `secs_in_period` and is fully decoupled from the visual setting.

---

## Changes by area

### A. The "Show Countdown" setting (relabel + invert + threshold)

- **Keep the internal config field `Config.hide_time`** and its serialization name unchanged
  (`refbox/src/config.rs:217`, migration at `:263`). No data migration needed — an existing
  `hide_time = true` config keeps working and simply displays as "SHOW COUNTDOWN = NO".
- **Invert at the UI only:** the button shows `bool_string(!hide_time)` as its value; toggling it
  still flips `hide_time` via `BoolGameParameter::HideTime`
  (`refbox/src/app/message.rs:810`, handler `refbox/src/app/mod.rs:~3517`). Add a short comment
  explaining the deliberate UI inversion.
- **Threshold 15 → 10** in the two break branches of `update_sender.rs`
  (`:510` and `:515`).
- **Translations:** remove the now-unused `hide-time-for-last-15-seconds` key and add
  `show-countdown-for-last-10-seconds = SHOW COUNTDOWN FOR LAST 10 SECONDS` across **all 15
  locales** (best-guess translations, native review later — no English placeholders).

### B. New "Audible Countdown" setting

- New field `Config.audible_countdown: bool` (default **false**) in `refbox/src/config.rs`, with
  migration defaulting false when absent (mirror the `get_boolean_value` pattern + a default test).
- New `BoolGameParameter::AudibleCountdown` variant (`message.rs`) and handler
  (`edited_settings.audible_countdown ^= true`) in `mod.rs`.
- **Translations:** add `audible-countdown-for-last-10-seconds = AUDIBLE COUNTDOWN FOR LAST 10
  SECONDS` across all 15 locales.

### C. App Options page layout (`view_builders/configuration.rs`, `make_app_config_page`)

- Row 4 becomes: `[ SHOW COUNTDOWN (left) , AUDIBLE COUNTDOWN (right) ]`.
- Row 5 becomes: `[ SHOW BEHIND TIME/DELAY (left) , horizontal_space() ]`.
- Both new buttons use `make_value_button` with `bool_string(...)`, matching their neighbours.
  (Verify the longer labels wrap acceptably in the compact button during walkthrough.)

### D. Display Options page layout (`view_builders/configuration.rs`, `make_display_config_page`)

- Remove the moved button from its row; keep `layout_btn` in place, with a `horizontal_space()`
  filling the vacated cell. No other Display control moves.

### E. New countdown beep sound

- Add a small generator script (mirroring the buzzer-upgrade `regen-buzzer-sounds.py` pattern,
  which lives only on that unmerged branch) under `refbox/resources/sounds/` that synthesizes a
  short single "pip" (~150 ms, clear mid-high tone) and writes `countdown.raw` (mono, 32-bit
  float LE, 44,100 Hz) — same format as the existing clips.
- Embed it in `sound_controller/sounds.rs` via `include_bytes!` + `process_array()`, add it to
  `SoundLibrary` and a `countdown()` accessor — mirroring the **whistle** (a fixed, dedicated
  sound, **not** added to the user-selectable buzzer list).
- Add `SoundMessage::TriggerCountdownBeep` (`button_handler/mod.rs`) and `SoundId::CountdownBeep`
  (`sound_controller/mod.rs`); play it one-shot (no loop, no scheduled fade-out) like the whistle,
  at the buzzer above/under-water volumes, gated by the master `sound_enabled` toggle. Add a
  public `SoundController::trigger_countdown_beep()` (mirror `trigger_whistle`).

### F. Trigger logic (`app/mod.rs`, `maybe_play_sound`)

In the `None` (normal-clock) branch, compute:

```text
is_break_before_play = current_period in
    { BetweenGames, HalfTime, PreOvertime, OvertimeHalfTime, PreSuddenDeath }
play_countdown = config.audible_countdown
    && is_break_before_play
    && new_snapshot.secs_in_period != self.snapshot.secs_in_period
    && (1..=10).contains(&new_snapshot.secs_in_period)
```

Fire `trigger_countdown_beep()` when `play_countdown`. It can never coincide with the 30-second
whistle or the 0-second buzzer, so it is an independent check.

---

## Edge cases (intended behaviour)

- Break shorter than 10 s, or period started manually without the break crossing 10→1: beep only
  for the seconds that actually occur (or not at all). Accepted — natural consequence of reading
  `secs_in_period`.
- Both visual Show-Countdown and Audible Countdown on/off independently; any combination is valid.

---

## Testing

- Config default for `audible_countdown` (false) + migration-absent test.
- Unit test for the `play_countdown` condition across the five break periods and the 10→1 window
  (and that it does NOT fire in playing periods, timeouts, or outside 1..=10).
- Confirm the 15→10 threshold change is reflected.
- `just check` (fmt, clippy -D warnings, tests, audit) green on all platforms.
- Walkthrough: launch the app, verify both page layouts, hear the 10 beeps before a half, and
  retune the beep tone if desired.

## Process / risk

Refbox-only, no state-machine or wire-format-structure changes → lean process. The threshold +
polarity touch a shipped visual feature and the sound path touches audio, so tests + a real
walkthrough (with audio) are required before PR.

## Multi-site propagation summary (for confirmation)

- **Translations, 15 locales:** remove `hide-time-for-last-15-seconds`; add
  `show-countdown-for-last-10-seconds` and `audible-countdown-for-last-10-seconds`.
- **Threshold literal `15` → `10`:** `update_sender.rs:510` and `:515`.
