# UPDATE AUDIO OUTPUT button — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "UPDATE AUDIO OUTPUT" button to the refbox Sound page that makes the running app re-adopt the operating system's *current* default audio output device, with no restart — on Windows/macOS only, absent on the Raspberry Pi.

**Architecture:** Pressing the button sends a new `SoundMessage::ReloadAudioOutput` to the sound-controller's background worker. The worker stops any playing sound, then rebuilds its `AudioContext` (and sound library) from scratch; a fresh context resolves the *current* OS default output device. The app's existing message channel to the controller is unchanged, so nothing else is rebuilt and the UI never blocks.

**Tech Stack:** Rust 2024, `iced` 0.13 (GUI), `web-audio-api` 1.2 (audio, `cpal` backend), `tokio` (async), `i18n-embed`/Fluent (`fl!` + `translations/<locale>/refbox.ftl`).

## Global Constraints

- **Crate scope:** `refbox` only. Do **not** touch `uwh-common`, `overlay`, `wireless-remote`, or any wire format.
- **Branch:** `feat/refbox/audio-output-button`, cut from `origin/master` (= tag `v0.4.3`, commit `e9c8890e`). Target release v0.4.4.
- **MSRV 1.85, edition 2024.** No language/std features newer than 1.85.
- **Clippy:** `cargo clippy -p refbox -- -D warnings` must pass (refbox is a bin crate — do **not** use `--all-targets`; that surfaces pre-existing test-code lints unrelated to this work).
- **No new dependencies.**
- **Literal label:** the button text is exactly **UPDATE AUDIO OUTPUT** (rendered across two lines to fit the button, same words).
- **All 15 locales required:** `de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN`. `refbox/build.rs` errors on a missing key in **release** builds. Non-English locales get a best-guess translation (no English placeholders); native review later.
- **Platform gating:** button present on `#[cfg(not(target_os = "linux"))]` (Windows + macOS); on Linux (the Pi, `aarch64-unknown-linux-gnu`) the Sound page keeps its existing empty spacer in that slot.
- **Testing reality:** the audio-device rebuild has no CI-testable seam (needs a real output device; builds an *online* context) and the button is compiled out on Linux/WSL. The per-task gate is `just check` passing; behavioural verification is **manual on the Windows laptop** that shows the bug. This is consistent with the project's lean-process rule for refbox UI/sound work.

---

### Task 1: Re-adopt-default mechanism + button (vertical slice)

This single task wires the whole feature for English. It must land as one slice because the pieces are interdependent for a clean `-D warnings` build: the button constructs `Message::UpdateAudioOutput`, the `update()` arm calls `reload_audio_output()`, and that method sends `SoundMessage::ReloadAudioOutput`. An unused enum variant or unused method would fail clippy in this bin crate, so producer and consumer ship together.

**Files:**
- Modify: `refbox/src/sound_controller/button_handler/mod.rs` (add `SoundMessage` variant)
- Modify: `refbox/src/sound_controller/mod.rs` (worker owns/rebuilds context; add `reload_audio_output`)
- Modify: `refbox/src/app/message.rs` (add `Message::UpdateAudioOutput`)
- Modify: `refbox/src/app/mod.rs` (handle the message in `update()`)
- Modify: `refbox/src/app/view_builders/configuration.rs` (add the gated button)
- Modify: `refbox/translations/en-US/refbox.ftl` (add the `update-audio-output` key)

**Interfaces:**
- Produces: `SoundMessage::ReloadAudioOutput` (unit variant); `SoundController::reload_audio_output(&self)`; `Message::UpdateAudioOutput` (unit variant); Fluent key `update-audio-output`.
- Consumes: existing `SoundController.msg_tx`, `make_button`, `light_gray_button`, `horizontal_space`, `fl!`, `Element<'a, Message>`.

- [ ] **Step 1: Add the `ReloadAudioOutput` message variant**

In `refbox/src/sound_controller/button_handler/mod.rs`, add the variant after `StopManualBuzzer,` (it is **not** cfg-gated — it is sent only on non-Linux, but compiling it everywhere keeps the worker's `match` uniform):

```rust
pub(super) enum SoundMessage {
    TriggerBuzzer,
    TriggerWhistle,
    StartManualBuzzer,
    StopManualBuzzer,
    ReloadAudioOutput,
    #[cfg(target_os = "linux")]
    StartWiredBuzzer,
    #[cfg(target_os = "linux")]
    StopWiredBuzzer,
    #[cfg(target_os = "linux")]
    WirelessRemoteReceived(RemoteId),
}
```

- [ ] **Step 2: Make the worker own its context/library, and stop pinning the context in the struct**

In `refbox/src/sound_controller/mod.rs`:

(a) Remove the `_context` field from the struct definition:

```rust
pub struct SoundController {
    msg_tx: UnboundedSender<SoundMessage>,
    settings_tx: Sender<SoundSettings>,
    stop_tx: Sender<bool>,
    handle: Option<JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    _button_handler: Option<ButtonHandler>,
    #[cfg(target_os = "linux")]
    remote_id_rx: Option<Receiver<RemoteId>>,
}
```

(b) Replace the clone-and-spawn preamble so the worker owns `context` and `library` mutably. Change:

```rust
        let mut _stop_rx = stop_rx.clone();
        let mut _settings_rx = settings_rx.clone();
        let _context = context.clone();

        let handle = task::spawn(async move {
            #[cfg_attr(not(target_os = "linux"), allow(unused_assignments))]
            let mut last_sound: Option<(SoundId, Sound)> = None;
```

to:

```rust
        let mut _stop_rx = stop_rx.clone();
        let mut _settings_rx = settings_rx.clone();

        let handle = task::spawn(async move {
            // Owned by the worker so the audio output device can be rebuilt in
            // place when the operator presses "UPDATE AUDIO OUTPUT". A fresh
            // AudioContext resolves the CURRENT system default output device.
            let mut context = context;
            let mut library = library;
            #[cfg_attr(not(target_os = "linux"), allow(unused_assignments))]
            let mut last_sound: Option<(SoundId, Sound)> = None;
```

(c) Update the five `Sound::new(_context.clone(), ...)` call sites inside `start_sound` to use the worker-owned binding. Replace **all** occurrences of `_context.clone()` with `context.clone()` (there are 5; two are inside `#[cfg(target_os = "linux")]` arms).

(d) Remove the `_context: context,` line from the `Self { ... }` constructor at the end of `new()`:

```rust
        Self {
            msg_tx,
            settings_tx,
            stop_tx,
            handle: Some(handle),
            #[cfg(target_os = "linux")]
            _button_handler,
            #[cfg(target_os = "linux")]
            remote_id_rx,
        }
```

- [ ] **Step 3: Handle `ReloadAudioOutput` in the worker loop**

In `refbox/src/sound_controller/mod.rs`, inside the worker's `match msg { ... }`, add this arm after the `SoundMessage::StopManualBuzzer => { ... }` arm:

```rust
                                    SoundMessage::ReloadAudioOutput => {
                                        info!("Reloading audio output to current system default");
                                        // Stop any sound playing on the OLD device first.
                                        if let Some((sound_id, sound)) = last_sound.take() {
                                            sound.stop().await;
                                            sound_ends.cancel(&sound_id);
                                        }
                                        // A fresh context (default sink) re-resolves
                                        // the device the OS currently has as default.
                                        let opts = AudioContextOptions {
                                            sample_rate: Some(SAMPLE_RATE),
                                            ..AudioContextOptions::default()
                                        };
                                        let new_context = AudioContext::new(opts);
                                        debug!(
                                            "Audio output reloaded with sink {:?}",
                                            new_context.sink_id()
                                        );
                                        context = Arc::new(new_context);
                                        library = SoundLibrary::new(&context);
                                    }
```

(No new `use` is needed: `info!`/`debug!` come from `log::*`; `AudioContext`/`AudioContextOptions` are already imported; `SAMPLE_RATE`/`SoundLibrary` come from `pub use sounds::*`; `Arc` from `std::sync`.)

- [ ] **Step 4: Add the `reload_audio_output` method**

In `refbox/src/sound_controller/mod.rs`, add this method to `impl SoundController` next to `trigger_buzzer` (mirror the existing `.unwrap()`-on-send style used by the sibling trigger methods):

```rust
    pub fn reload_audio_output(&self) {
        // The worker receiver lives for the app's lifetime; send only fails
        // after shutdown, when there is nothing left to play through anyway.
        self.msg_tx.send(SoundMessage::ReloadAudioOutput).unwrap()
    }
```

- [ ] **Step 5: Add the `Message::UpdateAudioOutput` variant**

In `refbox/src/app/message.rs`, add the unit variant after `RequestRemoteId,`:

```rust
    RequestRemoteId,
    UpdateAudioOutput,
```

- [ ] **Step 6: Handle the message in `update()`**

In `refbox/src/app/mod.rs`, add this arm immediately after the `Message::ShowGameDetails => { ... }` arm:

```rust
            Message::UpdateAudioOutput => {
                self.sound.reload_audio_output();
                Task::none()
            }
```

- [ ] **Step 7: Add the gated button to the Sound page**

In `refbox/src/app/view_builders/configuration.rs`, in `make_sound_config_page`, insert the slot definition right after the destructuring line:

```rust
    let EditableSettings { sound, .. } = settings;

    // Re-adopt the OS default output device. Laptop-only: the Pi runs Linux
    // with a fixed dedicated speaker, so the button is absent there and the
    // bottom row keeps its existing empty spacer.
    #[cfg(not(target_os = "linux"))]
    let audio_output_slot: Element<'a, Message> = make_button(fl!("update-audio-output"))
        .on_press(Message::UpdateAudioOutput)
        .style(light_gray_button)
        .into();
    #[cfg(target_os = "linux")]
    let audio_output_slot: Element<'a, Message> = horizontal_space().into();

    column![
```

Then, in the **bottom row** of that page, replace the **first** `horizontal_space(),` with `audio_output_slot,`:

```rust
        row![
            audio_output_slot,
            horizontal_space(),
            make_value_button(
                fl!("auto-sound-stop-play"),
                bool_string(sound.auto_sound_stop_play),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::AutoSoundStopPlay,
                    ))
                } else {
                    None
                },
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
```

- [ ] **Step 8: Add the English Fluent key**

In `refbox/translations/en-US/refbox.ftl`, add the key after the `manage-remotes = MANAGE REMOTES` line. The continuation line MUST be indented (Fluent requirement); this renders "UPDATE AUDIO" / "OUTPUT" on two lines so the literal label fits the button like its neighbours:

```
manage-remotes = MANAGE REMOTES
update-audio-output = UPDATE AUDIO
    OUTPUT
```

- [ ] **Step 9: Verify it builds clean**

Run: `just check`
Expected: PASS — fmt clean, `clippy -D warnings` clean, tests pass. (build.rs will emit a debug **warning** listing the 14 locales still missing `update-audio-output`; that is expected and is resolved in Task 2. It is a warning, not an error, in debug.)

If clippy flags `context`/`library` as "does not need to be mut", confirm Step 3's reassignment arm is present and compiled (it must be un-cfg'd).

- [ ] **Step 10: Commit**

```bash
git add refbox/src/sound_controller refbox/src/app refbox/translations/en-US/refbox.ftl
git commit -m "feat(refbox): add UPDATE AUDIO OUTPUT button to re-adopt OS default"
```

---

### Task 2: Translate the key into the remaining 14 locales

Mechanical completeness task. Required so **release** builds (Windows/macOS/Pi CI) don't error on the missing key. Each value is a best-guess two-line translation pending native review.

**Files:**
- Modify: `refbox/translations/{de-DE,es,fr,id-ID,it-IT,ja-JP,ko-KR,ms-MY,nl-NL,pt-PT,th-TH,tl-PH,tr-TR,zh-CN}/refbox.ftl`

- [ ] **Step 1: Add `update-audio-output` to each locale**

Add the matching entry below to each locale's `refbox.ftl` (place it near the other sound keys — exact position doesn't matter to `build.rs`, only presence). The continuation line MUST stay indented (4 spaces):

```
# de-DE
update-audio-output = AUDIOAUSGANG
    AKTUALISIEREN

# es
update-audio-output = ACTUALIZAR
    SALIDA DE AUDIO

# fr
update-audio-output = ACTUALISER
    SORTIE AUDIO

# id-ID
update-audio-output = PERBARUI
    OUTPUT AUDIO

# it-IT
update-audio-output = AGGIORNA
    USCITA AUDIO

# ja-JP
update-audio-output = オーディオ出力を
    更新

# ko-KR
update-audio-output = 오디오 출력
    업데이트

# ms-MY
update-audio-output = KEMAS KINI
    OUTPUT AUDIO

# nl-NL
update-audio-output = AUDIO-UITVOER
    BIJWERKEN

# pt-PT
update-audio-output = ATUALIZAR
    SAÍDA DE ÁUDIO

# th-TH
update-audio-output = อัปเดต
    เอาต์พุตเสียง

# tl-PH
update-audio-output = I-UPDATE ANG
    AUDIO OUTPUT

# tr-TR
update-audio-output = SES ÇIKIŞINI
    GÜNCELLE

# zh-CN
update-audio-output = 更新音频
    输出
```

- [ ] **Step 2: Verify no locale is missing the key**

Run: `just check`
Expected: PASS with **no** build.rs "Missing keys" warning for `update-audio-output` in any locale.

Cross-check the count:

Run: `rg -l "^update-audio-output" refbox/translations/*/refbox.ftl | wc -l`
Expected: `15`

- [ ] **Step 3: Commit**

```bash
git add refbox/translations
git commit -m "feat(refbox): translate UPDATE AUDIO OUTPUT into all locales"
```

---

## Manual verification (post-implementation, on the Windows laptop)

Not a code task — the acceptance test, run on the machine that shows the bug:

1. Build/run the refbox on the **Windows 11 laptop**, playing sound through the built-in speakers.
2. Connect the external speaker (aux/Bluetooth) and set it as the Windows **default output**.
3. On the Sound page, confirm **UPDATE AUDIO OUTPUT** appears directly below "Whistle Volume" (bottom row, left column), styled like MANAGE REMOTES, and press it.
4. Trigger a buzzer/whistle → it now plays through the **external** speaker, **no restart**. ✓
5. Switch the refbox UI language to a CJK locale (e.g. 中文) and confirm the button label renders without missing-glyph boxes. (Low risk: the button only renders on Windows/macOS, which have full system CJK fonts; the bundled-subset limitation is a Pi-only concern and the Pi has no button.)
6. Sanity: the button does not freeze the UI, and pressing it while a sound is playing stops it cleanly.

On a **Mac**, the same steps should behave identically. On a **Pi** build, confirm the button is **absent** and the Sound page layout is otherwise unchanged.

## Self-Review

**Spec coverage:**
- "Single UPDATE AUDIO OUTPUT button, re-adopt current default, no restart" → Task 1 (Steps 1–4 mechanism, 5–7 wiring/button). ✓
- "Directly below Whistle Volume (bottom row, col 1)" → Task 1 Step 7 (replace first `horizontal_space()`). ✓
- "Windows + Mac only, absent on Pi" → Task 1 Step 7 `#[cfg(not(target_os = "linux"))]` / Linux spacer. ✓
- "Rebuild on the worker, not the app; UI never blocks" → Task 1 Steps 2–4 (method only sends a message). ✓
- "All 15 locales, no English placeholders" → Task 1 Step 8 (en-US) + Task 2 (other 14). ✓
- "Old device released on reload" → Task 1 Step 2 removes the struct field that pinned the context; worker owns and reassigns it. ✓
- "Verification manual on Windows; not testable in CI" → Manual verification section + Global Constraints note. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code; the only intentional "exact position doesn't matter" is Task 2 placement, which `build.rs` confirms is presence-only. ✓

**Type consistency:** `SoundMessage::ReloadAudioOutput` (produced Step 1, sent Step 4, handled Step 3); `reload_audio_output(&self)` (defined Step 4, called Step 6); `Message::UpdateAudioOutput` (defined Step 5, constructed Step 7, handled Step 6); `update-audio-output` key (defined Step 8, referenced Step 7, completed Task 2). All names consistent. ✓
