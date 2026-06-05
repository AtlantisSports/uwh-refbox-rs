# Disable Player Display Brightness Button Without Panel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Grey out the Player Display Brightness button on the display-config settings page when the refbox was started without a real LED panel.

**Architecture:** A single conditional in one view builder. The brightness button's press action becomes `Some(...)` only when `has_led_panel` is true, otherwise `None`. The existing `make_value_button` helper already renders a `None` action as a greyed, non-pressable button that still shows its value.

**Tech Stack:** Rust 2024, `iced` 0.13, project `just` commands.

---

## Spec

See `docs/superpowers/specs/2026-06-05-brightness-button-disable-no-panel-design.md`.

## Process note (testing)

This is a mechanical `refbox` UI change with no new logic and no testable return value — view
builders return an opaque `iced` `Element`, so there is no clean unit-test seam, and per
`.claude/rules/plan-execution.md` (lean process) this class of change is verified by
compilation + `just check` + manual observation rather than an added automated test. No
behaviour changes when a panel *is* connected.

## File Structure

- Modify: `refbox/src/app/view_builders/configuration.rs` — the brightness button inside
  `make_display_config_page` (currently ~line 964). `has_led_panel` is already in scope here
  (the "Open New Display" button below it already uses it).

---

### Task 1: Make the brightness button's action conditional on `has_led_panel`

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (~line 1031 on master)

- [ ] **Step 1: Make the edit**

Find this exact block:

```rust
    let brightness_btn = make_value_button(
        fl!("player-display-brightness"),
        fl!("brightness", brightness = brightness.to_string()),
        (false, true),
        Some(Message::CycleParameter(CyclingParameter::Brightness)),
    );
```

Replace the action argument so it is conditional:

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

Note: the condition is the *opposite direction* from the "Open New Display" button just below
(that one disables when `has_led_panel` is true; brightness disables when it is false). Leave
"Open New Display" untouched.

- [ ] **Step 2: Build the crate**

Run: `cargo build -p refbox`
Expected: builds successfully, no errors.

- [ ] **Step 3: Lint (mirrors CI / `just lint` for this bin crate)**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: no warnings, no errors.

- [ ] **Step 4: Full project check**

Run: `just check`
Expected: fmt, lint, tests, audit all pass.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "fix(refbox): disable brightness button when no panel connected"
```

---

## Manual verification (operator-observable)

- Start the refbox normally (no LED panel):
  - WSL native: `WAYLAND_DISPLAY= cargo run -p refbox`
  - Open Settings → display page → confirm Player Display Brightness is greyed and does not
    respond to clicks, but still shows its current level (e.g. "Medium").
- Start the refbox with a serial port (panel present): `cargo run -p refbox -- --serial-port <port>`
  - Confirm the same button is fully usable and cycles brightness as before.

## Self-Review

- **Spec coverage:** Spec's single behaviour (disable when no panel, unchanged when panel
  present, using `has_led_panel`) is implemented by Task 1. No gaps.
- **Placeholder scan:** None — the full edited code block is shown.
- **Type consistency:** `has_led_panel: bool`, `Message::CycleParameter(CyclingParameter::Brightness)`,
  and `make_value_button(label, value, flags, Option<Message>)` all match the existing call site
  and the neighbouring "Open New Display" usage.
