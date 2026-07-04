# Restart Preserves CLI Args — Implementation Plan (Prerequisite for Self-Update)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the refbox's in-app restart relaunch the app with the **same command-line settings it was started with** (full-screen, serial/LED-panel port, real-hardware mode, ports, logging), stop silently swallowing a failed relaunch, and stop crashing startup when a serial port can't be opened.

**Architecture:** A new pure function `build_restart_argv(&Cli) -> Vec<String>` mirrors the existing `build_sim_argv` pattern and reconstructs the original argv (minus `--language`, which the persisted config must drive, and `--is-simulator`, which only applies to simulator children). `main()` calls it once right after parsing and uses the result in the existing `RESTART_PENDING` respawn block, now with error logging. Serial-port opening is changed from `.unwrap()` (panic) to graceful skip-and-log.

**Tech Stack:** Rust 2024, clap (`Cli` parser), tokio, tokio-serial, iced 0.13. MSRV 1.85. Validation: `cargo test -p refbox`, `cargo clippy -p refbox -- -D warnings` (mirrors `just lint`/CI for this bin crate — do NOT use `--all-targets`).

**Branch:** `fix/refbox/restart-preserve-cli-args`

**Scope boundary:** This plan fixes the *existing* restart mechanism only. It is the prerequisite for the separate self-update feature (`2026-06-10-refbox-self-update-design.md`), which is NOT implemented here. Hardware-sensitive resilience (SoundController shutdown ordering, port-acquisition retry/backoff) is intentionally deferred to the spare-Pi validation — see "Deferred: hardware-validated resilience" at the end.

**Why this matters (plain English):** Today, when the refbox restarts itself (after a language or mode change — and, in future, a self-update), it comes back **windowed, with no LED-panel connection, and in simulation mode**, because the relaunch forgets every start-up option. This fix makes a restart come back exactly as it was launched.

---

## File structure

- **`refbox/src/main.rs`** — add `build_restart_argv`, capture `restart_argv` after parse, use it in the respawn block with error logging, and add unit tests. (Owns: process startup, CLI, restart respawn.)
- **`refbox/src/app/update_sender.rs`** — make serial-port opening graceful (no panic). (Owns: serial/LED-panel + TCP output.)

No other files change. No `uwh-common` changes. No new dependencies.

---

## Task 1: `build_restart_argv` pure function + tests

**Files:**
- Modify: `refbox/src/main.rs` (add function near `build_sim_argv`, ~line 184; add tests near `sim_spawn_tests`, ~line 527)

- [ ] **Step 1: Write the failing tests**

Add a new test module at the end of `refbox/src/main.rs` (after the existing `sim_spawn_tests` module). Tests construct a `Cli` via clap's `parse_from` so we exercise the real parser:

```rust
#[cfg(test)]
mod restart_argv_tests {
    use super::*;

    fn argv_from(extra: &[&str]) -> Vec<String> {
        // First element is the program name, as clap expects.
        let mut argv = vec!["refbox"];
        argv.extend_from_slice(extra);
        let cli = Cli::parse_from(argv);
        build_restart_argv(&cli)
    }

    #[test]
    fn replays_fullscreen_only_when_set() {
        assert!(argv_from(&["--fullscreen"]).contains(&"--fullscreen".to_string()));
        assert!(!argv_from(&[]).contains(&"--fullscreen".to_string()));
    }

    #[test]
    fn replays_serial_port_and_baud_when_set() {
        let argv = argv_from(&["--serial-port", "/dev/ttyUSB0", "--baud-rate", "57600"]);
        assert!(argv.contains(&"--serial-port".to_string()));
        assert!(argv.contains(&"/dev/ttyUSB0".to_string()));
        assert!(argv.contains(&"--baud-rate".to_string()));
        assert!(argv.contains(&"57600".to_string()));
    }

    #[test]
    fn omits_serial_port_when_not_set() {
        let argv = argv_from(&[]);
        assert!(!argv.contains(&"--serial-port".to_string()));
    }

    #[test]
    fn replays_no_simulate_when_set() {
        assert!(argv_from(&["--no-simulate"]).contains(&"--no-simulate".to_string()));
        assert!(!argv_from(&[]).contains(&"--no-simulate".to_string()));
    }

    #[test]
    fn repeats_verbose_per_count() {
        let argv = argv_from(&["-v", "-v"]);
        assert_eq!(argv.iter().filter(|a| a.as_str() == "--verbose").count(), 2);
    }

    #[test]
    fn never_replays_language_or_is_simulator() {
        // --language must NOT be replayed: a restart is often triggered BY a
        // language change, and the new language lives in the persisted config.
        let argv = argv_from(&["--language", "fr", "--is-simulator"]);
        assert!(!argv.contains(&"--language".to_string()));
        assert!(!argv.contains(&"--is-simulator".to_string()));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox restart_argv_tests`
Expected: FAIL to compile with "cannot find function `build_restart_argv`".

- [ ] **Step 3: Implement `build_restart_argv`**

Add this function immediately after `build_sim_argv` (after ~line 212) in `refbox/src/main.rs`:

```rust
/// Build the argv used to relaunch the MAIN app on an in-app restart so the new
/// process inherits the same command-line settings the operator started with
/// (fullscreen, serial/LED-panel port, real-hardware mode, ports, logging).
///
/// Deliberately NOT replayed:
///  - `--language`: a restart is often triggered BY a language change. The new
///    language is persisted to the config file and must drive startup; replaying
///    the old `--language` would override it.
///  - `--is-simulator`: this relaunches the main app, never a simulator child.
pub fn build_restart_argv(args: &Cli) -> Vec<String> {
    let mut argv: Vec<String> = Vec::new();

    if args.no_simulate {
        argv.push("--no-simulate".to_string());
    }
    for _ in 0..args.verbose {
        argv.push("--verbose".to_string());
    }
    argv.push("--scale".to_string());
    argv.push(args.scale.to_string());
    if let Some(spacing) = args.spacing {
        argv.push("--spacing".to_string());
        argv.push(spacing.to_string());
    }
    if args.fullscreen {
        argv.push("--fullscreen".to_string());
    }
    argv.push("--binary-port".to_string());
    argv.push(args.binary_port.to_string());
    argv.push("--json-port".to_string());
    argv.push(args.json_port.to_string());
    if let Some(port) = &args.serial_port {
        argv.push("--serial-port".to_string());
        argv.push(port.clone());
        argv.push("--baud-rate".to_string());
        argv.push(args.baud_rate.to_string());
    }
    if args.allow_http {
        argv.push("--allow-http".to_string());
    }
    if args.all_events {
        argv.push("--all-events".to_string());
    }
    if let Some(loc) = &args.log_location {
        argv.push("--log-location".to_string());
        // A non-UTF-8 log path would already have panicked at startup, matching
        // the existing `build_sim_argv` behaviour.
        argv.push(loc.to_str().unwrap().to_string());
    }
    argv.push("--log-max-file-size".to_string());
    argv.push(args.log_max_file_size.to_string());
    argv.push("--num-old-logs".to_string());
    argv.push(args.num_old_logs.to_string());
    if args.simulate_sunlight_display {
        argv.push("--simulate-sunlight-display".to_string());
    }

    argv
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox restart_argv_tests`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/main.rs
git commit -m "feat(refbox): reconstruct restart argv from CLI args"
```

---

## Task 2: Use `restart_argv` in the respawn block + log failures

**Files:**
- Modify: `refbox/src/main.rs` (capture after parse ~line 226; respawn block ~line 518)

This task wires Task 1 into `main()`. It is exercised by the manual hardware validation (there is no unit test for spawning a real process).

- [ ] **Step 1: Capture the restart argv right after parsing**

In `main()`, immediately after `let args = Cli::parse();` (line ~225) and BEFORE any field of `args` is moved (the first move is `args.language` at line ~228), insert:

```rust
    // Capture the argv needed to relaunch this same configuration on an in-app
    // restart, before any field of `args` is moved below.
    let restart_argv = build_restart_argv(&args);
```

- [ ] **Step 2: Replace the respawn block to pass args and log failures**

Replace the existing respawn block (currently ~lines 518-522):

```rust
    if app::RESTART_PENDING.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
    }
```

with:

```rust
    if app::RESTART_PENDING.load(std::sync::atomic::Ordering::Relaxed) {
        match std::env::current_exe() {
            Ok(exe) => {
                info!("Restart requested: respawning {exe:?} with args {restart_argv:?}");
                if let Err(e) = std::process::Command::new(exe).args(&restart_argv).spawn() {
                    error!("Failed to respawn refbox on restart: {e}");
                }
            }
            Err(e) => error!("Failed to locate current exe for restart respawn: {e}"),
        }
    }
```

(`info!` and `error!` are already imported and used in this file; `std::env::current_exe` is already used.)

- [ ] **Step 3: Verify it builds and lint is clean**

Run: `cargo build -p refbox`
Expected: builds successfully.

Run: `cargo clippy -p refbox -- -D warnings`
Expected: no warnings.

- [ ] **Step 4: Verify existing tests still pass**

Run: `cargo test -p refbox`
Expected: PASS (including the Task 1 tests and the existing `sim_spawn_tests`).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/main.rs
git commit -m "fix(refbox): preserve CLI args and log failures on in-app restart"
```

---

## Task 3: Graceful serial-port open (no panic on a busy/missing port)

**Files:**
- Modify: `refbox/src/app/update_sender.rs` (extract `open_serial_ports`, used by `UpdateSender::new` ~line 47; add tests)

**Why:** `UpdateSender::new` currently does `.map(|builder| builder.open_native_async().unwrap())`. If the serial port is missing or still held by the just-exited process during a restart, this **panics and crashes startup**. It must instead log and continue (the LED panel is simply unavailable), which is also what makes a restart survive a brief port-handover overlap.

- [ ] **Step 1: Write the failing test**

Add to the test module in `refbox/src/app/update_sender.rs` (create a `#[cfg(test)] mod tests` at the end of the file if none exists). The open attempt registers with the tokio reactor, so the test runs on a tokio runtime:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn open_serial_ports_skips_ports_that_fail_to_open() {
        // A path that cannot exist as a serial device.
        let bad = tokio_serial::new("/dev/refbox_nonexistent_test_port", 115200);
        let opened = open_serial_ports(vec![bad]);
        assert!(opened.is_empty(), "an unopenable port must be skipped, not panic");
    }

    #[tokio::test]
    async fn open_serial_ports_empty_input_is_empty() {
        let opened = open_serial_ports(Vec::new());
        assert!(opened.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p refbox open_serial_ports`
Expected: FAIL to compile with "cannot find function `open_serial_ports`".

- [ ] **Step 3: Extract the helper and replace the `.unwrap()`**

In `refbox/src/app/update_sender.rs`, add this free function (above `impl UpdateSender`):

```rust
/// Open each serial port, skipping (and logging) any that fail to open instead
/// of panicking. A missing or still-held port simply means the LED panel is
/// unavailable — it must never crash startup or a restart.
fn open_serial_ports(builders: Vec<SerialPortBuilder>) -> Vec<SerialStream> {
    builders
        .into_iter()
        .filter_map(|builder| match builder.open_native_async() {
            Ok(port) => Some(port),
            Err(e) => {
                error!("Failed to open serial port; the LED panel will be unavailable: {e}");
                None
            }
        })
        .collect()
}
```

Then, in `UpdateSender::new`, replace:

```rust
        let initial = initial
            .into_iter()
            .map(|builder| builder.open_native_async().unwrap())
            .collect();
```

with:

```rust
        let initial = open_serial_ports(initial);
```

Ensure the needed imports exist at the top of `update_sender.rs`:
- `SerialStream` and `SerialPortBuilder` from `tokio_serial` (the file already uses `SerialPortBuilder`; add `SerialStream` to the same `use tokio_serial::{...}` if not present).
- The `error!` logging macro (the project uses `log`/`tracing`-style macros elsewhere; add `use log::error;` or match the import style already used in this file).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox open_serial_ports`
Expected: PASS (2 tests). If `open_native_async` requires a different invocation to attempt the open, keep the helper's signature and skip-on-error behaviour identical and adjust only the open call.

- [ ] **Step 5: Verify build + lint + full tests**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: no warnings.

Run: `cargo test -p refbox`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/update_sender.rs
git commit -m "fix(refbox): don't panic when a serial port can't be opened"
```

---

## Task 4: Manual hardware validation (merge gate)

**This is the hard gate. Do NOT merge until these pass on a real spare Pi.** Follows "Test A" of `2026-06-10-refbox-self-update-hardware-test-script.md`.

- [ ] **Step 1:** On a spare Pi wired like a tournament Pi (LED scoreboard on serial, wireless button), launch refbox exactly as the real Pi does (record the exact launch command/flags — full-screen, `--serial-port`, `--no-simulate`).
- [ ] **Step 2:** Confirm baseline: scoreboard shows the clock, window is full-screen, the wireless button sounds the buzzer.
- [ ] **Step 3:** Trigger a restart with no update by **changing the language** (to a language whose font differs, forcing the restart path) and confirming.
- [ ] **Step 4:** After relaunch, confirm ALL of: window returns **full-screen**; **scoreboard reconnects**; **buzzer still works**; logs go to the same location.
- [ ] **Step 5:** Repeat Step 3-4 by **changing the app mode** (the other restart trigger).
- [ ] **Step 6:** Record the results (pass/fail per check) in the PR description. Only if all pass is the branch ready to merge.

---

## Deferred: hardware-validated resilience (NOT in this branch)

These were flagged by review but their correct form depends on what the spare-Pi validation (Task 4) reveals. Do NOT implement blind here; address as a follow-up with evidence:

1. **SoundController shutdown ordering** — `SoundController::drop` (`refbox/src/sound_controller/mod.rs:~617`) calls `tokio::runtime::Handle::current().block_on(...)` inside `drop`. If Task 4 shows the app hangs or panics on exit, change `Handle::current()` to `Handle::try_current()` with a fallback that aborts the join handle, and/or bound the wait — but only after observing the actual behaviour on hardware.
2. **Port-acquisition race on restart** — if Task 4 shows the relaunched process intermittently misses the serial port or a TCP bind because the old process hasn't fully released it, add bounded retry/backoff (≈100 ms → 1 s, capped) around acquisition. Task 3 already prevents a *crash*; this would improve *reliability*.

These feed directly into the self-update plan's §9 open questions.

---

## Self-review notes

- **Spec coverage:** Implements spec §3 (the prerequisite). Items 1 (arg reconstruction), 2 (log spawn failure), and the serial-`.unwrap()` half of item 3 are coded; the retry/backoff and SoundController halves of item 3 are explicitly deferred to hardware validation with reasoning.
- **No placeholders:** all steps contain real code and exact commands.
- **Type consistency:** `build_restart_argv(&Cli) -> Vec<String>`, `open_serial_ports(Vec<SerialPortBuilder>) -> Vec<SerialStream>` used consistently across tasks.
