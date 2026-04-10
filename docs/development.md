# Development Guide

This guide explains how to set up your environment, run the software, and use the common
development tools. All commands use `just` — a task runner that wraps the underlying Rust
toolchain commands so you do not need to remember them.

---

## Prerequisites

Before working with this project, the following must be installed:

| Tool | Purpose | How to install |
|------|---------|----------------|
| Rust toolchain | Building and running the code | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `just` | Task runner (replaces remembering cargo commands) | `cargo install just` |
| `cargo-audit` | Security vulnerability scanner | `cargo install cargo-audit` |
| `cross` | Cross-compilation for Raspberry Pi | `cargo install cross` (requires Docker) |

The project requires **Rust 1.85 or newer** (MSRV). Run `rustup update` to get the latest.

---

## First-Time Setup

After cloning the repository:

```bash
cd uwh-refbox-rs
just install-hooks
```

This installs the pre-commit hook that checks branch names and code formatting before every
commit. You only need to do this once.

---

## Common Tasks

Run `just` with no arguments to see all available commands.

### Check everything (mirrors what CI runs)

```bash
just check
```

This runs formatting check, linting, tests, and security audit in sequence. **Run this before
opening a PR.** If any step fails, CI will also fail.

### Format code

```bash
just fmt
```

Automatically reformats all code to the project's style. Safe to run at any time — it only
changes whitespace and formatting, never logic.

### Check formatting without changing files

```bash
just fmt-check
```

Reports whether any files need formatting. Used by CI and the pre-commit hook.

### Run linting

```bash
just lint
```

Runs `clippy`, the Rust linter, across the entire workspace. All warnings are treated as errors.
Fix all warnings before committing.

### Run tests

```bash
just test
```

Runs all automated tests in the workspace.

### Security audit

```bash
just audit
```

Checks all dependencies for known security vulnerabilities.

### Build (debug)

```bash
just build
```

Builds the entire workspace in debug mode. Faster to compile, slower to run. Used during
development.

### Build (release)

```bash
just build-release
```

Builds in release mode. Slower to compile, faster to run. Used for tournament deployments.

### Build for Raspberry Pi

```bash
just build-rpi
```

Cross-compiles the `refbox` application for Raspberry Pi 4/5 (`aarch64` architecture). Requires
`cross` and Docker to be installed.

### Check the wireless-remote (embedded)

```bash
just check-wireless
```

Runs formatting and linting checks on the `wireless-remote` embedded firmware. This uses a
separate toolchain — see `docs/workspace-map.md` for why.

---

## Running the Refbox Locally

```bash
cargo run -p refbox
```

This starts the refbox GUI on your local machine. You can use it to test UI changes without
needing the full hardware setup (LED panel, wireless remote, etc.).

---

## What to Do When CI Fails

1. Look at the failing check on GitHub — the red X will link to the log
2. If it is a **formatting failure**: run `just fmt` locally, commit the result
3. If it is a **clippy failure**: run `just lint` locally to see the same errors, fix them
4. If it is an **audit failure**: run `just audit` to see the vulnerability, then ask Claude how
   to resolve it
5. If it is a **test failure**: run `just test` locally to reproduce it, then investigate

When in doubt, run `just check` locally and fix everything before pushing again.

---

## Cross-Compilation Notes

To build for Raspberry Pi, `cross` uses Docker to run a Linux ARM build environment. Docker must
be running before you use `just build-rpi`.

The target architecture is `aarch64-unknown-linux-gnu` (64-bit ARM Linux), which covers both
Raspberry Pi 4 and Raspberry Pi 5.

The `wireless-remote` has a completely separate build process — it targets the RP2040
microcontroller (`thumbv6m-none-eabi`) and uses a different Rust toolchain. Never run workspace
commands from inside the `wireless-remote/` directory.
