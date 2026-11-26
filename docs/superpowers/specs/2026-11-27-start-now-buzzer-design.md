# START NOW sounds the buzzer — design

**Status:** approved by the PO 2026-11-27. Branch `feat/refbox/start-now-buzzer`. Target v0.5.6.

## Intent

Eric, 2026-10-19: *"clicking "Start now" should trigger the alarm, not just start the time"*.
Teams in the water listen for the buzzer to know play has started. When the operator skips the rest of a
break with START NOW, play starts in silence today.

## Ruling (PO, 2026-11-27, option 1 of 3)

START NOW plays exactly what the break running out would have played:

- the operator's chosen buzzer sound (the AutoBuzzer, `sound.trigger_buzzer()`);
- in all five breaks where START NOW shows: between games, half-time, pre-overtime, overtime half-time,
  pre-sudden-death;
- only when **AUTO SOUND START PLAY** (`auto_sound_start_play`) is on;
- silent when **SOUND ENABLED** is off (already enforced at playback in `sound_controller`).

Rejected: ignoring AUTO SOUND START PLAY (option 2, would surprise operators who switched it off); only
between games (option 3, inconsistent across breaks).

## How it works today

- START NOW = `Message::StartPlayNow` (`main_view.rs`), handled in `refbox/src/app/mod.rs`: it calls
  `tm.start_play_now(now)`, then `apply_snapshot`.
- The 0:00 buzzer comes from `maybe_play_sound`, which buzzes only on the tick where a break's
  `secs_in_period` reaches 0, using `period_end_sounds(period).0` ("the end of this period starts play")
  and `auto_sound_start_play`. START NOW skips that tick.
- `start_play_now` refuses (returns an error) during a timeout, in a play period, or between games with no
  startable next game. START NOW is the only path into it; the wireless remote does not send it.

## Approaches

1. **Chosen — buzz in the `StartPlayNow` handler.** Read the engine's period before the call; on success,
   if that period's end starts play and AUTO SOUND START PLAY is on, trigger the buzzer. The decision is a
   pure function so a test can pin it (a test cannot observe the sound or the returned Task).
2. Teach `maybe_play_sound` to spot a break→play transition that skipped 0:00. Rejected: that function runs
   on every tick and already distinguishes many cases; it would also see the natural 0:00 transition and
   risk a double buzz.
3. Buzz from the engine (`tournament_manager`). Rejected: the engine owns no sound; all sound decisions live
   in the app.

## Design

- New pure function next to `period_end_sounds`, e.g.
  `fn start_now_plays_buzzer(left_period: GamePeriod, auto_sound_start_play: bool) -> bool`, true exactly
  when `period_end_sounds(left_period, _).0 && auto_sound_start_play`.
- In the `StartPlayNow` arm: capture `tm.current_period()` before `start_play_now`; on `Err` return as today
  (no sound, so a tap that loses the race with 0:00 cannot double-buzz); on `Ok`, if the function says so,
  log `info!("START NOW: triggering buzzer")` and call `self.sound.trigger_buzzer()`.
- No double buzz on success: the snapshot after START NOW is a play period at full time, which
  `maybe_play_sound` does not buzz.

## Goals / non-goals

- Goal: the buzzer on START NOW in all five breaks, honouring both sound settings.
- Non-goals: the ALARM button, the buzzer settings, the sound engine, which breaks show START NOW or when it
  is greyed, the beep-test page's own START, the stale-tap guard (separate branch).

## Verification

- Unit test of the pure function: true for all five breaks with the setting on; false with it off; false
  for every play period.
- Walkthrough: tap START NOW between games and at half-time with AUTO SOUND START PLAY on (buzzer, and the
  log line); repeat with it off (silence, no log line).
