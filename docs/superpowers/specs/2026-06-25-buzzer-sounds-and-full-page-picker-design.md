# Buzzer Sounds + Full-Page Picker — Design Spec

**Date:** 2026-06-25
**Status:** Design approved (pending spec review)
**Crate scope:** `refbox` only

---

## 1. Goal & motivation

Two related changes to how the operator chooses the buzzer sound:

1. **More buzzer options.** At a tournament, multiple courts run within earshot of
   each other. When two nearby games use the same buzzer, players can't tell whether
   a buzzer is for *their* game. We add new sounds chosen to be **mutually
   distinguishable** (different pitch register, rhythm, timbre) so each nearby court
   can pick a clearly different one.

2. **Full-page picker instead of the carousel.** Today the buzzer is chosen via a
   single tile you tap to cycle through the options one at a time. We replace that
   with a **full-page picker** — a tap-to-select grid with the selection highlighted —
   matching the existing **Languages** page in layout, style, and behavior. The picker
   is reachable from **both** the main Sound settings page **and** the beep test's
   Sound settings page (the beep test already mirrors the Languages picker this way).

The picker footer adds a **Test** button so the operator can audition the selected
sound before applying.

## 2. Scope boundary

- **In scope:** `refbox` only — the sound engine (`src/sound_controller/`), the
  settings UI (`src/app/view_builders/configuration.rs`,
  `src/app/view_builders/beep_test_settings.rs`), the message/state plumbing
  (`src/app/message.rs`, `src/app/mod.rs`), embedded sound assets
  (`refbox/resources/sounds/`), and translation strings (`refbox/translations/`).
- **Explicitly NOT touching:**
  - `uwh-common`, the LED-panel / wireless-remote / overlay wire formats, and the
    game clock / tournament-manager state machine. The buzzer sound has no effect on
    game timing, so the time-engine golden traces are unaffected.
  - The `wireless-remote` firmware (no changes; the remote already sends a button id
    and the refbox maps it to a `BuzzerSound`).
  - The **whistle** sound and its settings (separate from the buzzer; out of scope).
  - The Languages page itself (it is the template we copy, not a target of change).
  - Any *other* carousel picker (volumes, etc.) — only the buzzer picker is converted.

## 3. The sound set — 12 total

The 5 existing sounds are unchanged. We add **7 new** sounds. Order in the picker:
existing 5 first (familiar), then the 7 new ones — laid out as **3 rows of 4** with
room to grow.

| # | Name | Status | Character |
|---|------|--------|-----------|
| 1 | Buzz | existing | Harsh sustained ~500 Hz (default) |
| 2 | Whoop | existing | Sustained ~900 Hz |
| 3 | Crazy | existing | Chaotic warble |
| 4 | De De Du | existing | Three rising notes |
| 5 | Two Tone | existing | Narrow fast alternation 800/1000 Hz |
| 6 | Airhorn | **new** | Semi/truck air horn — brassy repeated blasts |
| 7 | Pipes | **new** | Single metallic clang, rapid even banging (~470 Hz) |
| 8 | Klaxon | **new** | Old-car "ah-OO-gah" rising/falling honk |
| 9 | Pip | **new** | Even fast beeps (~1700 Hz, older-ear friendly) |
| 10 | Pulse | **new** | Slow low pulse (~330 Hz) — only low-register tone |
| 11 | Siren | **new** | Continuous up/down wail 500↔1500 Hz |
| 12 | Trill | **new** | Very fast shimmery trill between two close tones |

*(Triad — a three-falling-note tone — was prototyped and dropped to land on a clean
3×4 grid.)*

### 3.1 Sound assets

The new sounds are **synthesized**, not recordings — this guarantees they are mutually
distinct, loop seamlessly, and have no licensing constraints. They were auditioned and
approved via a browser sound board.

Each sound is embedded as a **single-cycle loop element** in the same format the app
already uses: **mono, 32-bit float, little-endian, 44,100 Hz** `.raw` files in
`refbox/resources/sounds/` (the same format as `buzz.raw`, `whoop.raw`, …). The app
loops the element to fill the playback window, so each element is designed to:

- **Fit the ~2.15s auto window:** the engine already plays the whole number of loop
  cycles nearest 2.15s (`whole_cycles_for`), so a sound is never cut mid-pattern. The
  new elements are tuned so this lands at ~2.0–2.15s (3 cycles for most).
- **Loop seamlessly:** the loop seam is either in silence (even-rhythm patterns) or
  phase-continuous (sustained tones / sweeps), so the repeat boundary is imperceptible
  when the buzzer is held (manual alarm / remote). Verified by measuring the seam
  discontinuity against the sound's own internal sample-to-sample motion.

The synthesis recipe (frequencies, durations, envelopes, seam handling) lives in a
reproducible script, **`refbox/resources/sounds/regen-buzzer-sounds.py`** (to be added
alongside the existing font `regen-cjk-font.py` precedent). The 7 embed-ready `.raw`
element files have already been generated and are staged for copy-in.

| New element | Loop period | Cycles in ~2.15s | Auto length |
|---|---|---|---|
| Airhorn | 700 ms | 3 | 2.10 s |
| Pipes | 215 ms | 10 | 2.15 s |
| Klaxon | 700 ms | 3 | 2.10 s |
| Pip | 140 ms | 15 | 2.10 s |
| Pulse | 700 ms | 3 | 2.10 s |
| Siren | 700 ms | 3 | 2.10 s |
| Trill | 500 ms | 4 | 2.00 s |

## 4. Acceptance criteria (operator-observable)

1. The main **Sound settings** page shows a **BUZZER** tile displaying the current
   selection (e.g. "BUZZER / SIREN"); tapping it opens the full-page picker.
2. The picker shows **all 12 sounds** as a tap-to-select grid (3 rows of 4), the
   selected sound highlighted blue, matching the Languages page look.
3. The picker footer is **Cancel (red) | Test | Apply (green)**. Apply is grayed until
   the selection changes; Cancel discards and returns to the Sound page; Apply saves
   and returns.
4. **Test** plays the **currently-selected** sound once, as the full ~2.15s buzzer,
   through the app's real audio path (so it reflects what the game will play).
5. Selecting any **new** sound, applying, and triggering the buzzer (period end or
   manual alarm) plays that sound; held alarm loops it with no perceptible seam.
6. The **beep test** Sound settings page has the same BUZZER tile → identical full-page
   picker (Cancel | Test | Apply), wired into the beep-test settings flow, and its
   selection is honored when the beep test plays its buzzer.
7. The **BUZZER** and **TEST** button labels are translated in all 15 locales. Sound
   names display in English in every locale (consistent with the existing sounds).
8. `just check` passes (fmt, clippy `-D warnings`, tests, audit).

## 5. Architectural sketch

Mirrors two existing patterns: the **Languages picker** (`ConfigPage::Language` +
`make_language_select_page`) for the main side, and the beep test's **copy** of it
(`BeepTestConfigPage::Language` + `build_beep_test_language_picker`) for the beep-test
side. The buzzer picker is the same shape with buzzer-specific wiring.

### 5.1 Sounds — `src/sound_controller/sounds.rs`
- Add 7 variants to `enum BuzzerSound` (Airhorn, Pipes, Klaxon, Pip, Pulse, Siren,
  Trill). The `EnumFromStr!` macro keeps config parsing working; new variants are
  additive and backward-compatible with existing saved configs.
- Add the embedded `static [f32; N]` arrays + `include_bytes!` for the 7 `.raw` files.
- Extend `SoundLibrary` (fields, `Index<BuzzerSound>` match, `new()` buffer copies).
- Extend `impl Display for BuzzerSound` with the English names ("Airhorn", "Pipes",
  "Klaxon", "Pip", "Pulse", "Siren", "Trill").
- The picker's cycle/`Cyclable` impl in `configuration.rs` is **removed** for the
  buzzer (no longer cycled); selection is direct.

### 5.2 Test playback — `src/sound_controller/mod.rs`
- Today the worker can only auto-play the **saved** `settings.buzzer_sound`. Add a way
  to play an **arbitrary** `BuzzerSound` once as the timed (~2.15s) auto-buzzer:
  a new `SoundMessage::TestBuzzer(BuzzerSound)` handled like `TriggerBuzzer` but using
  the passed sound, and a `SoundController::test_buzzer(sound)` method.
- Test playback uses the existing timed/looped path (`Sound::new(.., repeat=true,
  timed=true)`), so it sounds exactly like the real auto-buzzer and self-stops.
- It queues through the same `sound_queue`, so a live auto-buzzer/manual alarm still
  takes precedence; Test is only reachable from a settings page.

### 5.3 Main picker — `src/app/view_builders/configuration.rs` + `message.rs` + `mod.rs`
- `ConfigPage::Buzzer` (new variant in `message.rs`).
- `make_sound_config_page`: replace the buzzer carousel tile with a **value-style
  button** showing the current sound that fires `ChangeConfigPage(ConfigPage::Buzzer)`.
- `make_buzzer_select_page`: new view builder mirroring `make_language_select_page` —
  game-time banner, 3×4 grid of `make_value_button`-style selectable cells
  (`blue_selected_button` for the chosen one), then a **Cancel | Test | Apply** footer.
- Staging mirrors language: add `pending_buzzer` / `original_buzzer` to
  `EditableSettings`, a `PageEntrySnapshot::Buzzer { original, pending }` variant, and
  seed both on entry (handler for `ChangeConfigPage(ConfigPage::Buzzer)`).
- Messages: `SelectBuzzer(BuzzerSound)` (sets pending), `TestBuzzer` (plays pending via
  5.2), `BuzzerSelectComplete { canceled: bool }` (Apply commits `pending` into
  `config.sound.buzzer_sound` + persists + applies to the controller; Cancel discards;
  both return to `ConfigPage::Sound`). `navigate_to_parent(ConfigPage::Buzzer)` →
  `ConfigPage::Sound`.

### 5.4 Beep-test picker — `src/app/view_builders/beep_test_settings.rs` + plumbing
- `BeepTestConfigPage::Buzzer` (new variant).
- `build_beep_test_sound_settings_page`: replace the buzzer carousel tile with the same
  value-style button → opens the beep-test buzzer picker.
- `build_beep_test_buzzer_picker`: new view builder, functionally identical to the main
  picker, using the beep-test footer (`make_beep_test_cancel_apply_footer` extended to
  include Test, or a buzzer-specific footer) and beep-test messages:
  `BeepTestEditOpenBuzzer`, `BeepTestSelectBuzzer(BuzzerSound)`, `BeepTestTestBuzzer`,
  `BeepTestBuzzerSave` / `BeepTestBuzzerCancel`. Selection is staged in
  `edited_settings.sound.buzzer_sound` and saved like the existing beep-test Sound
  Settings sub-page.

### 5.5 Translations — `refbox/translations/`
- Reuse the existing `buzzer-sound` key ("BUZZER") for the entry tile label.
- Add a new `test` key ("TEST") and translate it in all 15 locales (per project rule:
  no English placeholders).
- Sound names are **not** localized (they come from `Display`, like the existing
  sounds).

## 6. Risks & notes
- **Sound engine is sensitive.** Changes in `sound_controller` get the heavier care
  (manual audio walkthrough) even though no wire format is involved.
- **Config back-compat:** new `BuzzerSound` variants are additive; old configs keep
  parsing. Per-remote sound overrides (`RemoteInfo.sound: Option<BuzzerSound>`) gain the
  new options automatically.
- **Test vs. live audio:** Test queues through the same controller; verify it can't
  step on a live game buzzer (it shouldn't, since the picker is settings-only, but
  confirm in the walkthrough).
- **Binary size:** 7 small `.raw` elements (~0.6 MB total) embedded at build time —
  negligible.

## 7. Rough task list (refined in the implementation plan)
1. Add `regen-buzzer-sounds.py` + the 7 `.raw` files to `refbox/resources/sounds/`.
2. `sounds.rs`: enum variants, embeds, `SoundLibrary`, `Display`.
3. `sound_controller/mod.rs`: `TestBuzzer` message + `test_buzzer()` method.
4. `message.rs`: `ConfigPage::Buzzer`, `BeepTestConfigPage::Buzzer`, and the new
   `Message` variants (main + beep-test + Test).
5. `configuration.rs`: entry tile + `make_buzzer_select_page` + remove buzzer cycling.
6. `beep_test_settings.rs`: entry tile + `build_beep_test_buzzer_picker`.
7. `mod.rs`: navigation, staging/seed, snapshot variant, Apply/Cancel/Test handlers
   (both hierarchies).
8. Translations: `test` key in 15 locales.
9. `just check`; in-app walkthrough (both entry points, Test, held-alarm seam, each new
   sound).

## 8. Verification plan
- Unit/build: `just check` green.
- In-app walkthrough (operator-observable, both entry points):
  - Main Sound page → BUZZER tile shows current sound → opens picker.
  - Pick each new sound, press **Test**, confirm it plays the full ~2.15s buzzer.
  - Apply, trigger a real period-end buzzer + a held manual alarm; confirm the held
    alarm loops with no audible seam.
  - Cancel discards; Apply gray until changed.
  - Repeat via the beep test Sound settings page; confirm the beep test plays the
    chosen sound at lap end.
