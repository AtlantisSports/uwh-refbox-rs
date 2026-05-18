# Sound Controller Audit — Pre-Deletion Gate

**Date:** 2026-05-19
**Purpose:** Confirm beep-test's `sound_controller` is fully covered by refbox's
`sound_controller` before deleting `beep-test/` in Task 12.

## Methodology

Surveyed every call site in `beep-test/src/` that touches the `sound_controller` public API,
then located the equivalent in `refbox/src/sound_controller/`. The audit covers the surface
beep-test's cadence engine and main loop call into — not the internals of beep-test's
`sound_controller` module itself, which is being absorbed by deletion.

The two primary consumers in beep-test are:
- `beep-test/src/app/mod.rs` — the BeepTestApp main loop; constructs the controller and
  calls it directly on sound events.
- `beep-test/src/tournament_manager/mod.rs` — the cadence engine; does **not** call the
  sound controller directly. It produces a snapshot; `maybe_play_sound()` in `app/mod.rs`
  reads that snapshot and then calls the sound API. All sound call sites are in `app/mod.rs`.

## API surface used by beep-test (cadence engine + main loop)

| API call | beep-test location | refbox equivalent | Signature match | Notes |
|----------|--------------------|-------------------|-----------------|-------|
| `SoundController::new(settings, trigger_flash)` | `beep-test/src/sound_controller/mod.rs:193` | `refbox/src/sound_controller/mod.rs:296` | Near-identical — see note | Refbox adds `+ Sync` to the `trigger_flash` bound and takes `settings` as `mut`. Same call site in `app/mod.rs:210`. |
| `.trigger_whistle()` | `beep-test/src/app/mod.rs:86` (called from `maybe_play_sound`) | `refbox/src/sound_controller/mod.rs:576` | Yes — `pub fn trigger_whistle(&self)` in both | Identical method name and `&self` receiver. |
| `.trigger_buzzer()` | `beep-test/src/app/mod.rs:89` (called from `maybe_play_sound`) | `refbox/src/sound_controller/mod.rs:588` | Yes — `pub fn trigger_buzzer(&self)` in both | Identical method name and `&self` receiver. |
| `.update_settings(settings)` | `beep-test/src/app/mod.rs:98` (called from `apply_settings_change`) | `refbox/src/sound_controller/mod.rs:572` | Yes — `pub fn update_settings(&self, settings: SoundSettings)` in both | Identical signature. |
| `SoundSettings` (struct, serialization) | `beep-test/src/sound_controller/mod.rs:34–51` | `refbox/src/sound_controller/mod.rs:63–81` | Near-identical — see note | Refbox adds one field: `manual_alarm_enabled: bool` (default `false`). All fields beep-test uses are present with the same names and types. |
| `Volume` (enum) | `beep-test/src/sound_controller/mod.rs:148–157` | `refbox/src/sound_controller/mod.rs:183–194` | Yes | Same five variants (`Off`, `Low`, `Medium`, `High`, `Max`) and same `as_f32()` scale values. Refbox adds a `Display` impl that goes through the translation system; beep-test uses `EnumDisplay!` from `macro_attr`. Both serialize/deserialize identically. |
| `BuzzerSound` (enum) | `beep-test/src/sound_controller/sounds.rs:52–62` | `refbox/src/sound_controller/sounds.rs:51–62` | Yes — byte-for-byte identical | Same five variants, same defaults, same `Display` strings. |
| `RemoteInfo` (struct) | `beep-test/src/sound_controller/mod.rs:171–175` | `refbox/src/sound_controller/mod.rs:220–224` | Structurally equivalent — see note | The `id` field type differs: beep-test uses `u32`; refbox uses `RemoteId` (a newtype wrapper over `u32`, with `impl From<u32>`). Beep-test's `SoundSettings.remotes` is not used by the cadence engine or its sound triggers at all — it is only used by the wireless remote hardware path, which Task 7 (UI integration) will handle via refbox's own remote handling. |
| `SoundSettings::migrate(old: &Table)` | `beep-test/src/sound_controller/mod.rs:53–144` | `refbox/src/sound_controller/mod.rs:83–181` | Semantically equivalent | Refbox's version also migrates `manual_alarm_enabled`. All other fields are migrated identically. |

## Constructor trait-bound divergence (detail)

Beep-test's `SoundController::new`:
```
F: Send + Fn() -> Result<(), tokio::sync::mpsc::error::TrySendError<ServerMessage>> + 'static
```

Refbox's `SoundController::new`:
```
F: Send + Sync + Fn() -> Result<(), tokio::sync::mpsc::error::TrySendError<ServerMessage>> + 'static
```

Refbox requires `Sync` on `trigger_flash`. This is a stricter bound. When Task 7 wires the
absorbed cadence engine into `RefBoxApp` using refbox's sound controller (which is what it will
do — beep-test's controller is deleted), the `trigger_flash` closure passed will be the same
one already used in refbox's main game loop. That closure already satisfies `Sync` (it is
already compiled and tested in refbox). This is not a gap — it is the absorbed caller adopting
the target crate's stricter bound, which is correct behaviour.

## `RemoteInfo.id` type divergence (detail)

In beep-test, `RemoteInfo.id` is `u32`. In refbox, `RemoteInfo.id` is `RemoteId` (a newtype
wrapper). The cadence engine's `maybe_play_sound()` in `app/mod.rs` does not inspect remote
IDs at all — it only reads `snapshot.secs_in_period` and `current_period` to decide whether to
call `trigger_whistle()` or `trigger_buzzer()`. The `remotes` field of `SoundSettings` is
consumed only by refbox's own hardware button handler (LoRa / wired button path), which is
already in refbox and will continue to work unchanged. This is not a gap.

## `SoundSettings.manual_alarm_enabled` field (detail)

Refbox's `SoundSettings` has an extra boolean field not present in beep-test: `manual_alarm_enabled`.
It defaults to `false`. The absorbed beep-test mode will not use a manual alarm — the cadence
engine's sound logic only calls `trigger_whistle()` and `trigger_buzzer()`. When the absorbed
mode constructs a `SoundSettings` value (from the saved config or from defaults), the extra
field will default to `false` and have no effect on the two sound triggers the cadence engine
uses. This is not a gap.

## Sound duration constant divergence (detail)

Beep-test: `const SOUND_LEN: f64 = 2.0`
Refbox:    `const SOUND_LEN: f64 = 2.15`

The refbox value was updated to 2.15s (from 2.0s) to avoid a software fade-out landing on a
loop boundary of the buzz/whoop sounds. The longer value is strictly better for the beep-test
use case. This is not a gap; it is a quality improvement the absorbed code inherits for free.

## Gaps

None — refbox's `sound_controller` provides every API surface beep-test's cadence engine and
main loop use, with matching or stricter semantics. The three noted divergences (constructor
`Sync` bound, `RemoteInfo.id` newtype, `manual_alarm_enabled` field) are all benign: the
absorbed caller will automatically satisfy the stricter bound, the remote-ID type is not used
by the cadence engine's sound path, and the extra field defaults harmlessly to `false`.

Deletion of `beep-test/src/sound_controller/` is safe.

## Conclusion

**Deletion gate PASSED.** Task 12 may proceed.

The absorbed cadence engine running inside refbox will call `trigger_whistle()` and
`trigger_buzzer()` on refbox's `SoundController`, both of which exist with identical signatures
and identical internal semantics. `update_settings()` is also present and identical. All public
types the call sites use (`SoundSettings`, `Volume`, `BuzzerSound`) are present in refbox with
matching variants, serialization, and behaviour.
