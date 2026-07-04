# Sounds Options Page — Layout Reorder

**Date:** 2026-06-17
**Crate:** `refbox` (UI only)
**Type:** UI layout change (no behaviour change)

## Goal

Rearrange the controls on the refbox **Sounds** options page into a new
3-column grid order. This is a pure reordering of the ten existing controls.
No control is added, removed, or rewired; no enable/disable gating changes;
no config fields, messages, or translation keys change.

## Scope boundary

- **In scope:** the cell order inside `make_sound_config_page` in
  `refbox/src/app/view_builders/configuration.rs`.
- **Out of scope:** what any control does, its label/value text, its
  enable/disable gating logic, the Cancel/Apply footer, the BeepTest sound
  page (`build_beep_test_sound_settings_page`), and every other options
  sub-page.

## Target layout

| | Col 1 | Col 2 | Col 3 |
|---|---|---|---|
| Row 1 | Sound Enabled | Buzzer Sound | Manage Remotes |
| Row 2 | Whistle Enabled | Above Water Volume | Alarm Button |
| Row 3 | Whistle Volume | Underwater Volume | Auto Sound Start Play |
| Row 4 | *(blank)* | *(blank)* | Auto Sound Stop Play |
| Footer | Cancel | | Apply |

The lone control in Row 4 (Auto Sound Stop Play) sits in **Col 3 (right)**,
directly under Auto Sound Start Play. The two empty cells (Col 1, Col 2) are
filled with `horizontal_space()`, the same idiom the page already uses.

## Controls (carried verbatim)

Each control keeps its exact label key, value expression, and message wiring,
including gating:

| Control | Label key | Message / gating |
|---|---|---|
| Sound Enabled | `sound-enabled` | `ToggleBoolParameter(SoundEnabled)` — always enabled |
| Whistle Enabled | `whistle-enabled` | `ToggleBoolParameter(RefAlertEnabled)` — gated on `sound_enabled` |
| Whistle Volume | `whistle-volume` | `CycleParameter(AlertVolume)` — gated on `sound_enabled && whistle_enabled` |
| Buzzer Sound | `buzzer-sound` | `CycleParameter(BuzzerSound)` — gated on `sound_enabled` |
| Above Water Volume | `above-water-volume` | `CycleParameter(AboveWaterVol)` — gated on `sound_enabled` |
| Underwater Volume | `underwater-volume` | `CycleParameter(UnderWaterVol)` — gated on `sound_enabled` |
| Manage Remotes | `manage-remotes` | `ChangeConfigPage(Remotes(0, false))` — always enabled |
| Alarm Button | `alarm-button` | `ToggleBoolParameter(ManualAlarmEnabled)` — gated on `sound_enabled` |
| Auto Sound Start Play | `auto-sound-start-play` | `ToggleBoolParameter(AutoSoundStartPlay)` — gated on `sound_enabled` |
| Auto Sound Stop Play | `auto-sound-stop-play` | `ToggleBoolParameter(AutoSoundStopPlay)` — gated on `sound_enabled` |

## Acceptance criteria

- The Sounds page renders the four rows above, in the stated cell order.
- Every control behaves exactly as before (same toggle/cycle action, same
  disabled-when-sound-off behaviour).
- `cargo build -p refbox` and `cargo clippy -p refbox -- -D warnings` are clean.
- Visible by launching refbox and opening Settings → Sounds.

## Notes

Local working doc — not committed to the feature branch / PR (per project
convention that spec/plan docs stay local).
