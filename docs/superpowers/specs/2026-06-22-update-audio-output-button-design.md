# Design — "UPDATE AUDIO OUTPUT" button (re-adopt current system default speaker)

- **Date:** 2026-06-22
- **Target release:** v0.4.4
- **Base:** branch off `master` (= tag `v0.4.3`, commit `e9c8890e`)
- **Crate(s):** `refbox` only
- **Process:** lean (refbox UI + sound-controller; no `uwh-common`, no wire format, no wireless-remote)
- **Status:** design — awaiting user review before writing the implementation plan

## Scope Card (recap)

**Task:** Add an "UPDATE AUDIO OUTPUT" button on the Sound Options page (directly below "Whistle
Volume", Row 5 / Column 1) that makes the refbox re-adopt whatever output device the operating
system currently has set as default, taking effect immediately without a restart. Targeted for
v0.4.4.

**Explicitly not doing:**
- Not changing how sounds are generated, the buzzer/whistle/alarm timing, or the volume logic.
- Not building an in-app device-picker list, and not auto-following OS default-device changes
  (this is an explicit operator-triggered "catch up to the OS default" button).
- Not adding output-device selection to the LED panel, the stream overlay, or the wireless remote.
- Not touching the Raspberry Pi sound setup (the button is absent on Pi builds).

## Problem

The refbox opens **one** audio output device — whatever the OS has set as the default *at the
moment the app launches* — and holds it for the whole session. It never follows a later change of
the default. So if an operator connects an external speaker (aux/Bluetooth) *after* the refbox is
already running, the refbox keeps playing to the built-in speakers, while a browser (YouTube)
follows the new default. Confirmed in code: the device is resolved once in
`AudioContext::new` (`web-audio-api` `io/cpal.rs` picks `host.default_output_device()` only when
the sink id is empty, which is our case) and the controller is created exactly once at
`refbox/src/app/mod.rs:1456`.

This bites only on **laptops** (Windows/macOS) where audio devices are swapped during use. On the
**Raspberry Pi** the speaker is dedicated, present at boot, and never swapped, so the issue does
not arise there in practice.

## Approach (chosen)

A single **action button** labelled **`UPDATE AUDIO OUTPUT`** on the Sound page. Pressing it tells
the sound system to **rebuild its audio connection**, which re-resolves the current OS default
output device and starts playing through it immediately.

**Why a rebuild rather than a "switch device" call:** the engine's `set_sink_id_sync("")` is a
no-op when the context is already on the default sink (which it always is for us), so it cannot
re-adopt a *changed* default. A freshly created `AudioContext` (default sink) always resolves the
*current* default device — so re-creating the context is the reliable mechanism.

**Where the rebuild happens — inside the sound-controller worker, not the app:**
The app holds the controller as `App.sound: SoundController` and talks to it only through a message
channel. Rather than drop/recreate the whole controller from `update()` (which would run the
controller's blocking `Drop` on the UI thread and require re-wiring the app's channels), the
controller's own background worker rebuilds its audio context in place when it receives a new
message. The app just sends that message — non-blocking, no other part of the app changes.

### Rejected alternatives
- **In-app device-picker page (a scrolling list of speakers with Cancel/Apply).** More code and
  risk; device ids are documented as *not stable across sessions* (so names would have to be
  persisted and re-matched); largely duplicates the OS's own output picker. The operator already
  picks the speaker in the OS — this button just makes the refbox honour that choice.
- **Rebuild the whole `SoundController` from `App::update()`.** Simpler to write but runs the
  controller's 2-second-bounded blocking `Drop` on the UI thread and forces the app to re-bind the
  controller's message channels. The in-worker rebuild avoids both. (Kept as a fallback only if the
  in-worker rebuild proves impractical during implementation.)

## UX / placement

The shipping (v0.4.3) Sound page is a tidy 3-column grid. "Whistle Volume" sits at **Row 4 /
Column 1**; the slot directly beneath it (**Row 5 / Column 1**) is currently an empty
`horizontal_space()`. The new button drops into that empty slot:

```
│ WHISTLE <vol>│ UNDERWATER <vol> │   AUTO SOUND (start) on   │   Row 4
├──────────────┼──────────────────┼──────────────────────────┤
│ UPDATE AUDIO │  (empty)         │   AUTO SOUND (stop)  on    │   Row 5
│   OUTPUT     │                  │                            │
                    [ CANCEL ]      [ APPLY ]
```

- **Label (literal):** `UPDATE AUDIO OUTPUT` — matches the existing all-caps button style
  (`MANAGE REMOTES`, `ALARM`).
- **Style:** mirror the existing `MANAGE REMOTES` action button (`make_button(...).style(light_gray_button)`),
  read its actual construction at implementation time and match element-for-element.
- **Always pressable:** like `MANAGE REMOTES`, it is *not* gated on the `SOUND` on/off toggle — it
  is a device-routing/setup action independent of whether sounds are currently enabled.
- **Feedback:** none in the core scope — the operator confirms by triggering a buzzer/whistle and
  hearing it from the new speaker. (Optional future enhancement: play a short confirmation beep
  after the switch. Deferred — adds timing/edge-case complexity for marginal value.)

## Platform gating

- **Windows and macOS builds:** button present and active (same root issue applies to both).
- **Raspberry Pi build:** button **absent**. The Pi is `target_os = "linux"`
  (`aarch64-unknown-linux-gnu`), consistent with the existing `#[cfg(target_os = "linux")]`
  convention in the sound controller. On Linux, Row 5 / Column 1 keeps its existing empty
  `horizontal_space()`, so the Pi layout and sound behaviour are unchanged.
- Gate the button with `#[cfg(not(target_os = "linux"))]`; provide the empty spacer under
  `#[cfg(target_os = "linux")]`.

## Architecture / files to change

1. **`refbox/src/sound_controller/button_handler/mod.rs`** — add a `ReloadAudioOutput` variant to
   the `SoundMessage` enum (always present; only ever sent on non-Linux). 
2. **`refbox/src/sound_controller/mod.rs`**
   - Add `SoundController::reload_audio_output(&self)` that sends `SoundMessage::ReloadAudioOutput`
     on `msg_tx`.
   - Restructure the worker so its audio `context` and `library` are rebindable. On
     `ReloadAudioOutput`: stop/clear any currently-playing sound, then rebuild
     `context = Arc::new(AudioContext::new(opts))` and `library = SoundLibrary::new(&context)`
     (re-using `SAMPLE_RATE`/`AudioContextOptions`). The new context resolves the current OS
     default device.
   - Decide the fate of the struct's `_context` field: prefer letting the worker own the context so
     the *old* context (and its hold on the old device) is released on reload, rather than being
     pinned by the field. Confirm at implementation time that nothing else relies on the field.
3. **`refbox/src/app/message.rs`** — add `Message::UpdateAudioOutput`.
4. **`refbox/src/app/mod.rs`** — handle `Message::UpdateAudioOutput` in `update()` by calling
   `self.sound.reload_audio_output()` (non-blocking).
5. **`refbox/src/app/view_builders/configuration.rs`** — in `make_sound_config_page`, replace the
   first `horizontal_space()` of the bottom row (Row 5 / Col 1) with the gated button.
6. **Translations** — add key `update-audio-output = UPDATE AUDIO OUTPUT` to **all 15 locales**
   (`de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR,
   zh-CN`) with a best-guess translation per project policy (no English placeholders; native review
   later). "AUDIO OUTPUT" is a UI/technical term — keep it recognisable per locale conventions.

## Acceptance criteria (observable)

1. On a **Windows or macOS** laptop, with the refbox already running and sounding through the
   built-in speakers: connect an external speaker, set it as the OS default output, press
   **UPDATE AUDIO OUTPUT** → the next buzzer/whistle plays through the **external** speaker, with
   **no app restart**.
2. The button appears on the Sound page **directly below "Whistle Volume"** (Row 5 / Col 1) on
   Windows/macOS builds, styled like `MANAGE REMOTES`.
3. On the **Raspberry Pi** build the button is **absent** and the rest of the Sound page layout is
   visually unchanged.
4. Pressing the button does not crash, hang the UI, or leave the refbox unable to play sound;
   a sound playing at the moment of the press is stopped cleanly.
5. `just check` passes (fmt, clippy `-D warnings`, tests) on all platforms.

## Testing & verification

- **Primary verification is manual, on the Windows laptop** that exhibits the bug (and ideally a
  Mac), following criterion 1. The audio-device rebuild cannot be meaningfully unit-tested in CI
  (no real output device, and it builds an *online* context).
- **Note / limitation:** because the button is compiled out on Linux, it **cannot be walked
  through on the WSL/Linux dev machine** — WSL's audio routing would not be representative anyway.
  Plan to validate on the actual Windows laptop.
- Automated coverage is limited to compilation + clippy + existing sound-controller tests
  (settings serialization, `whole_cycles_for`, bounded-shutdown). No new behavioural unit test is
  proposed for the rebuild itself; if a cheap non-audio assertion is feasible (e.g. the worker
  handles the new message without panicking under an offline/headless setup), add it, otherwise
  rely on manual verification.

## Risks

- **Worker rebuild correctness:** rebuilding the context mid-session must not panic, deadlock, or
  drop the worker; current sound must be stopped first. This is the main risk and the focus of
  implementation care (sound controller is the delicate area here, though blast radius stays inside
  `refbox`).
- **Audible artifact on switch:** opening a new output stream may produce a brief click. Acceptable
  for a deliberate setup action; verify it is not disruptive.
- **No CI safety net for the audio path:** mitigated by manual laptop verification before merge.
