# Design: Disable Player Display Brightness button when no panel is connected

Date: 2026-06-05
Crate: `refbox` (only)

## Problem

The display-config settings page has a **Player Display Brightness** button. Brightness only
affects the physical LED scoreboard panel. When the refbox is started without a real panel, the
button does nothing useful, yet it is still pressable. We intended to grey it out in the last
display-modes PR set, but the change did not land. We want it in before publishing v0.4.2.

## Behaviour

- When the refbox is started **without** a real LED panel, the Player Display Brightness button
  is greyed out and non-pressable. It still shows the current brightness level (e.g. "Medium")
  — it just cannot be changed.
- When the refbox is started **with** a real panel, the button works exactly as it does today.

"Panel connected" uses the existing, settled convention: the `has_led_panel` flag, which is
`true` when the refbox was launched with `--serial-port`. This is the same signal the
neighbouring "Open New Display" button already keys off. No new or live detection is added.

## Implementation

Single change, single file: `refbox/src/app/view_builders/configuration.rs`, the
`brightness_btn` binding in `make_display_config_page` (~line 1031 on master).

Today the button always passes a press action:

```rust
let brightness_btn = make_value_button(
    fl!("player-display-brightness"),
    fl!("brightness", brightness = brightness.to_string()),
    (false, true),
    Some(Message::CycleParameter(CyclingParameter::Brightness)),
);
```

Make the action conditional on `has_led_panel`:

```rust
let brightness_btn = make_value_button(
    fl!("player-display-brightness"),
    fl!("brightness", brightness = brightness.to_string()),
    (false, true),
    if has_led_panel {
        Some(Message::CycleParameter(CyclingParameter::Brightness))
    } else {
        None
    },
);
```

`make_value_button` already renders a `None` action as a greyed-out, non-pressable button that
still displays its value — the identical idiom used by the disabled-volume buttons on the sound
page. `has_led_panel` is already in scope in this function (the "Open New Display" block uses it).

Note the condition direction is the *opposite* of "Open New Display": that button is disabled
when a panel *is* present (it can't open a sim window then); brightness is disabled when a panel
is *absent*.

## Scope boundary

- Only `refbox`. One file, one button.
- Not touching `uwh-common`, the wire format, or `wireless-remote`.
- Not changing brightness behaviour when a panel *is* connected.
- Not changing any other settings button or the page layout.
- Not adding live plug/unplug detection.

## How to verify

- Start the refbox normally (no LED panel) → Settings → display page → the Player Display
  Brightness button is greyed and does not respond to clicks, but still shows its level.
- Start the refbox with `--serial-port` (real panel) → the same button is fully usable.

## Process note

Lean process (per `.claude/rules/plan-execution.md`): `refbox` UI, low blast radius. Compilation
plus `just check` is sufficient; no heavy per-task verification ceremony required.
