# Embedded Target Rules

These rules govern work on the `wireless-remote` embedded firmware. This is the highest-risk
area of the codebase — mistakes here cannot be caught by CI and require physical hardware to
test and fix.

## What the Wireless Remote Is

The wireless remote is a small handheld button device used by the referee during a game. When the
referee presses the button, the device sends a radio signal (LoRa) to the refbox computer, which
plays the appropriate buzzer sound. The firmware for this device runs on an RP2040 microcontroller
(the same chip as the Raspberry Pi Pico).

## The Critical Difference

The `wireless-remote/` directory is **NOT part of the main Cargo workspace**. It has:
- Its own `Cargo.toml` (a separate workspace root)
- Its own `rust-toolchain.toml` (a different Rust toolchain component set)
- Its own target architecture: `thumbv6m-none-eabi` (ARM Cortex-M0+, no operating system)

This means normal workspace commands (`cargo build --workspace`, `just check`, etc.) do **not**
apply to it. Running workspace commands from inside `wireless-remote/` with the wrong toolchain
will produce confusing errors.

## Rules

**Never make changes to `wireless-remote` without explicit discussion with the human first.**
Firmware changes require physical hardware to test. A mistake here means the device stops working
and must be reflashed — which requires the correct hardware and setup.

**Never run workspace cargo commands from the `wireless-remote/` directory.** Use
`just check-wireless` from the workspace root to check the wireless-remote, or navigate to
the `wireless-remote/` directory and use its own toolchain explicitly.

**Use `defmt` for all logging.** This is the embedded-appropriate logging framework. Do not use
`log`, `env_logger`, or `println!` — these are not available in a no-OS environment.

**Use Embassy async patterns only.** The firmware uses the Embassy embedded async framework.
Do not introduce blocking loops or RTOS-style patterns that conflict with Embassy's executor.

**Document hardware validation.** Whenever a change is made to `wireless-remote`, record:
- What was changed
- What hardware testing was done (or explicitly state that no hardware testing was done)
- What the expected observable behaviour change is

## Shared Code: `wireless-modes`

The `wireless-modes` crate defines the LoRa communication modes shared between `refbox` and
`wireless-remote`. This crate IS part of the main workspace but directly affects the wireless
remote's behaviour. Changes to `wireless-modes` must be considered in the context of both sides
of the radio link.

## When to Escalate

If a task seems to require changes to `wireless-remote` but the scope was not explicitly stated
as embedded firmware work, stop and ask the human to confirm before proceeding.
