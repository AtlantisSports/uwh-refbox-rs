# Refbox Restart Resilience Hardening (Phase 1) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing in-app restart survive the brief window where the exiting process still holds the LED-panel serial port, the overlay TCP ports, or the audio thread — so a restart reliably comes back full-screen with the scoreboard and buzzer working.

**Architecture:** Three independent, low-blast-radius hardening changes in the `refbox` crate, all building on the merged PR #1073 restart (`build_restart_argv`): (1) bounded retry when opening the serial port and binding the overlay TCP ports during startup; (2) a firm time-limit on the sound controller's blocking shutdown so a hung audio teardown cannot prevent the relaunch; (3) an exhaustive-destructure guard so future CLI flags can't silently be dropped from restarts. Each change is pure-function-testable; the real timing is validated on the spare Pi before publish (Phase 1 of `docs/superpowers/specs/2026-06-15-refbox-self-update-phased-design.md`).

**Tech Stack:** Rust 2024 / MSRV 1.85; `tokio` (async runtime, `time::timeout`/`sleep`), `tokio_serial` (serial open), `clap` (`Cli`), `cargo test -p refbox`, `cargo clippy -p refbox -- -D warnings`, `just check`.

---

## Pre-flight notes (read before Task 1)

- **Branch point is `origin/master` (post-PR #1073)** — NOT the current working branch
  `feat/refbox/time-golden-trace-spike`, which predates #1073 and lacks `build_restart_argv`.
  Always `git fetch origin master` first (`feedback_fetch_master_before_branching`).
- **Approval gates (project rule):** creating the branch, every `git commit`, and opening the PR
  require the human's approval. The per-task `git commit` steps below are the intended commit
  points; the executing session pauses for approval at each, per `.claude/rules/communication.md`.
- **This plan + the design spec are local working docs** (`reference_plan_docs_not_committed`):
  never `git add` them to the branch or include them in the PR.
- **Heavy process** (`.claude/rules/plan-execution.md`): this touches the restart path, hardware
  acquisition, and the sound controller — verify each task (`cargo test -p refbox`) before moving on.
- **Hardware-validation checkpoints** are marked `🔧 PI:` — they are confirmed on the spare Pi
  during the 5×-restart test before publish, not in CI.
- **refbox is bin-only** (`reference_refbox_bin_crate_clippy_scope`): use `cargo test -p refbox`
  and `cargo clippy -p refbox -- -D warnings` (no `--all-targets`).

## File structure

| File | Change |
|------|--------|
| `refbox/src/app/update_sender.rs` | Serial transient-error predicate + per-port retry (Task 2); TCP transient-error predicate + `bind_with_retry` + wire into `listener_loop` (Task 3) |
| `refbox/src/sound_controller/mod.rs` | Bounded shutdown helper + use it in `Drop` (Task 4) |
| `refbox/src/main.rs` | Exhaustive-destructure drift guard in `build_restart_argv` (Task 5) |

No new files, no new dependencies, no public-API changes.

---

## Task 1: Branch from master and confirm a clean baseline

**Files:** none (git + verification only)

- [ ] **Step 1: Fetch master and create the branch** *(approval-gated)*

```bash
git fetch origin master
git switch -c fix/refbox/restart-resilience origin/master
```

- [ ] **Step 2: Confirm the baseline is green**

Run: `cargo test -p refbox`
Expected: PASS (this is the post-#1073 baseline; `restart_argv_tests` and
`open_serial_ports_*` tests pass).

- [ ] **Step 3: Confirm `build_restart_argv` is present (sanity that we branched correctly)**

Run: `grep -n "fn build_restart_argv" refbox/src/main.rs`
Expected: one match (~line 230). If absent, you branched from the wrong base — stop.

---

## Task 2: Bounded retry for opening the serial port

**Why:** During a restart the exiting process may still hold the LED-panel serial port for a
moment. Today `open_serial_ports` skips-and-logs on the first failure, so the scoreboard silently
stays dark. Retry briefly on a *transient* failure; skip immediately when the device is simply
absent (so dev laptops and no-panel Pis pay no startup penalty).

**Files:**
- Modify: `refbox/src/app/update_sender.rs` (`open_serial_ports`, ~lines 24-38; imports ~line 19-21; tests ~line 1075)

- [ ] **Step 1: Write the failing predicate test**

Add inside the existing `#[cfg(test)] mod tests` in `refbox/src/app/update_sender.rs` (near the
`open_serial_ports_*` tests at the bottom):

```rust
    #[test]
    fn transient_serial_error_retries_busy_but_not_absent() {
        use tokio_serial::{Error as SerialError, ErrorKind as SerialErrorKind};
        // A momentarily-held port (typical during a restart) is worth retrying.
        let busy = SerialError::new(
            SerialErrorKind::Io(std::io::ErrorKind::PermissionDenied),
            "busy",
        );
        assert!(is_transient_serial_error(&busy));
        // An absent / unknown device must be skipped immediately, not retried.
        let absent = SerialError::new(SerialErrorKind::NoDevice, "no device");
        assert!(!is_transient_serial_error(&absent));
    }
```

- [ ] **Step 2: Run it to verify it fails (does not compile — `is_transient_serial_error` undefined)**

Run: `cargo test -p refbox transient_serial_error_retries_busy_but_not_absent`
Expected: FAIL — `cannot find function is_transient_serial_error`.

- [ ] **Step 3: Implement the predicate and the retry**

Replace the existing `open_serial_ports` function (lines 24-38) with:

```rust
/// Total time budget for retrying a *transient* serial-open failure, plus the
/// initial backoff step (doubled each attempt, capped by the budget). Sized for
/// the brief window during a restart where the exiting process is still
/// releasing the port.
/// 🔧 PI: confirm/tune on the spare Pi during the 5×-restart test.
const SERIAL_OPEN_RETRY_BUDGET: Duration = Duration::from_millis(2000);
const SERIAL_OPEN_RETRY_INITIAL: Duration = Duration::from_millis(100);

/// Whether a serial-open error is worth retrying. `true` for a device that
/// exists but is momentarily unavailable (e.g. still held by the exiting
/// process during a restart); `false` for an absent/misconfigured device, which
/// would only delay startup if retried.
/// 🔧 PI: if a briefly-held port on the Pi surfaces a different `ErrorKind`,
/// widen this allowlist (this is the single place to tune it).
fn is_transient_serial_error(e: &tokio_serial::Error) -> bool {
    use tokio_serial::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::Io(std::io::ErrorKind::PermissionDenied)
            | ErrorKind::Io(std::io::ErrorKind::WouldBlock)
            | ErrorKind::Io(std::io::ErrorKind::TimedOut)
    )
}

/// Open one serial port, retrying transient failures within a bounded budget,
/// and skipping (logging) a permanent failure instead of panicking. A missing
/// or unopenable port simply means the LED panel is unavailable — it must never
/// crash startup or a restart.
fn open_one_serial_port_with_retry(builder: SerialPortBuilder) -> Option<SerialStream> {
    let deadline = std::time::Instant::now() + SERIAL_OPEN_RETRY_BUDGET;
    let mut backoff = SERIAL_OPEN_RETRY_INITIAL;
    loop {
        match builder.clone().open_native_async() {
            Ok(port) => return Some(port),
            Err(e) => {
                if is_transient_serial_error(&e) && std::time::Instant::now() < deadline {
                    warn!("Serial port busy, retrying in {backoff:?}: {e}");
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(SERIAL_OPEN_RETRY_BUDGET);
                    continue;
                }
                error!("Failed to open serial port; the LED panel will be unavailable: {e}");
                return None;
            }
        }
    }
}

/// Open each serial port (see `open_one_serial_port_with_retry`).
fn open_serial_ports(builders: Vec<SerialPortBuilder>) -> Vec<SerialStream> {
    builders
        .into_iter()
        .filter_map(open_one_serial_port_with_retry)
        .collect()
}
```

Then ensure `io` and `warn` are usable: `io` is already imported (`use tokio::io;` at line 14)
and `warn`/`error` come from `use log::*;` (line 2). `Duration` is already imported (line 19).
No import edits needed.

- [ ] **Step 4: Run the predicate test (passes) and the existing serial tests (still fast)**

Run: `cargo test -p refbox open_serial_ports_ transient_serial_error_`
Expected: PASS. `open_serial_ports_skips_ports_that_fail_to_open` (nonexistent port → absent →
not transient) still returns immediately; no multi-second hang.

- [ ] **Step 5: Lint and commit** *(approval-gated)*

```bash
cargo clippy -p refbox -- -D warnings
git add refbox/src/app/update_sender.rs
git commit -m "fix(refbox): retry transient serial-port open during restart"
```

> 🔧 PI: the retry *loop's* real exercise (a port briefly held by the exiting process) only happens
> on hardware. Unit tests cover the predicate and the no-retry paths; the loop is validated in the
> 5×-restart Pi test.

---

## Task 3: Bounded retry for binding the overlay TCP ports

**Why:** Same overlap race for the two overlay TCP ports. On a restart the exiting process may
still hold the IPv6 listener; today the bind fails once and the overlay silently can't connect.
Retry the **primary (IPv6) binds** on `AddrInUse`. Do **NOT** retry the opportunistic IPv4 binds —
on Linux the IPv6 `::` bind is dual-stack, so the IPv4 bind legitimately fails `AddrInUse` every
boot and retrying it would add a fixed startup delay.

**Files:**
- Modify: `refbox/src/app/update_sender.rs` (imports ~line 19; new helper + predicate near
  `listener_loop`, ~line 630; the four bind lines 632-650; tests at bottom)

- [ ] **Step 1: Add `sleep` to the tokio time import**

Change the import at line 19 from:

```rust
    time::{Duration, Instant, sleep_until, timeout},
```
to:
```rust
    time::{Duration, Instant, sleep, sleep_until, timeout},
```

- [ ] **Step 2: Write the failing behavioural test**

Add inside `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn transient_bind_error_only_for_addr_in_use() {
        assert!(is_transient_bind_error(&std::io::Error::from(
            std::io::ErrorKind::AddrInUse
        )));
        assert!(!is_transient_bind_error(&std::io::Error::from(
            std::io::ErrorKind::PermissionDenied
        )));
    }

    #[tokio::test]
    async fn bind_with_retry_gives_up_on_held_port_within_budget() {
        // Hold an ephemeral port, then try to bind it again: the second bind
        // fails AddrInUse, so bind_with_retry must retry then give up (None)
        // within roughly its budget — never hang or panic.
        let held = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = held.local_addr().unwrap().port();
        let start = Instant::now();
        let result = bind_with_retry(
            ("127.0.0.1", port),
            "test",
            Duration::from_millis(150),
            Duration::from_millis(50),
        )
        .await;
        assert!(result.is_none(), "a held port must yield None, not a listener");
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "must give up within roughly the budget"
        );
    }
```

- [ ] **Step 3: Run it to verify it fails (undefined functions)**

Run: `cargo test -p refbox bind_with_retry_gives_up_on_held_port_within_budget transient_bind_error_only_for_addr_in_use`
Expected: FAIL — `cannot find function is_transient_bind_error` / `bind_with_retry`.

- [ ] **Step 4: Implement the predicate and the helper**

Add immediately **above** `async fn listener_loop(...)` (line 630) in `update_sender.rs`:

```rust
/// Time budget + initial backoff for retrying a transient (port-in-use) bind.
/// 🔧 PI: confirm/tune on the spare Pi during the 5×-restart test.
const TCP_BIND_RETRY_BUDGET: Duration = Duration::from_millis(2000);
const TCP_BIND_RETRY_INITIAL: Duration = Duration::from_millis(100);

/// A bind failure is worth retrying only when the address is momentarily still
/// in use (e.g. held by the exiting process during a restart).
fn is_transient_bind_error(e: &io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::AddrInUse)
}

/// Bind a TCP listener, retrying an `AddrInUse` failure within `budget` (with
/// exponential backoff from `initial`) before giving up and returning `None`.
/// Any non-transient error gives up immediately. Never panics.
async fn bind_with_retry(
    addr: (&str, u16),
    label: &str,
    budget: Duration,
    initial: Duration,
) -> Option<TcpListener> {
    let deadline = Instant::now() + budget;
    let mut backoff = initial;
    loop {
        match TcpListener::bind(addr).await {
            Ok(listener) => return Some(listener),
            Err(e) => {
                if is_transient_bind_error(&e) && Instant::now() < deadline {
                    warn!("{label} port {} in use, retrying in {backoff:?}", addr.1);
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(budget);
                    continue;
                }
                error!("Failed to bind {label} port {}: {e:?}", addr.1);
                return None;
            }
        }
    }
}
```

- [ ] **Step 5: Wire the primary binds through the helper**

In `listener_loop`, replace the two IPv6 `match TcpListener::bind(...)` blocks (lines 632-645) with:

```rust
    let binary_listener_v6 = bind_with_retry(
        ("::", binary_port),
        "binary",
        TCP_BIND_RETRY_BUDGET,
        TCP_BIND_RETRY_INITIAL,
    )
    .await;
    let json_listener_v6 = bind_with_retry(
        ("::", json_port),
        "JSON",
        TCP_BIND_RETRY_BUDGET,
        TCP_BIND_RETRY_INITIAL,
    )
    .await;
```

Leave the two IPv4 `.ok()` binds (lines 649-650) unchanged — see the "Why" note above.

- [ ] **Step 6: Run the tests (pass)**

Run: `cargo test -p refbox bind_with_retry_gives_up_on_held_port_within_budget transient_bind_error_only_for_addr_in_use`
Expected: PASS, completing in well under a second.

- [ ] **Step 7: Lint and commit** *(approval-gated)*

```bash
cargo clippy -p refbox -- -D warnings
git add refbox/src/app/update_sender.rs
git commit -m "fix(refbox): retry transient overlay TCP bind during restart"
```

---

## Task 4: Time-limit the sound controller's blocking shutdown

**Why:** `SoundController`'s `Drop` does `block_on(handle.await)` during teardown, and teardown
runs **before** the relaunch spawn in `main`. If the audio thread hangs, the relaunch never fires —
the old window closes and nothing comes back. Bound the wait so a hung shutdown can't strand the
operator; proceed with the restart on timeout.

**Files:**
- Modify: `refbox/src/sound_controller/mod.rs` (import ~line 24; `Drop` impl, lines 611-625; tests)

- [ ] **Step 1: Add `timeout` to the tokio time import**

Change the import at line 24 from:

```rust
    time::{Duration, Instant, sleep, sleep_until},
```
to:
```rust
    time::{Duration, Instant, sleep, sleep_until, timeout},
```

- [ ] **Step 2: Write the failing test**

Add a `#[cfg(test)]` module at the end of `refbox/src/sound_controller/mod.rs` (or into the
existing test module if one is present — check the file end first):

```rust
#[cfg(test)]
mod shutdown_tests {
    use super::*;

    #[tokio::test]
    async fn await_handle_bounded_returns_for_hung_task() {
        // A worker that never finishes must not block teardown past the bound.
        let handle = tokio::spawn(async { std::future::pending::<()>().await });
        let start = Instant::now();
        await_handle_bounded(handle, Duration::from_millis(150)).await;
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "bounded shutdown must return shortly after the timeout"
        );
    }

    #[tokio::test]
    async fn await_handle_bounded_returns_promptly_for_finished_task() {
        let handle = tokio::spawn(async {});
        let start = Instant::now();
        await_handle_bounded(handle, Duration::from_secs(5)).await;
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
```

- [ ] **Step 3: Run it to verify it fails (undefined function)**

Run: `cargo test -p refbox await_handle_bounded_returns_for_hung_task`
Expected: FAIL — `cannot find function await_handle_bounded`.

- [ ] **Step 4: Implement the helper and use it in `Drop`**

Add this free function just above `impl Drop for SoundController` (line 611):

```rust
/// Maximum time the sound-controller teardown waits for its worker to finish
/// before proceeding with shutdown/restart anyway. A hung audio shutdown must
/// never be able to prevent the app from relaunching.
/// 🔧 PI: confirm on the spare Pi during the 5×-restart test.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Await the worker `JoinHandle`, but never longer than `limit`. On timeout,
/// log and return so teardown (and the pending relaunch) can proceed.
async fn await_handle_bounded(handle: JoinHandle<()>, limit: Duration) {
    match timeout(limit, handle).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("Sound controller thread failed: {e}"),
        Err(_) => warn!("Sound controller shutdown timed out after {limit:?}; proceeding"),
    }
}
```

Then replace the body of `Drop` (lines 611-625) with:

```rust
impl Drop for SoundController {
    fn drop(&mut self) {
        if self.stop_tx.send(true).is_err() {
            return;
        }

        if let Some(handle) = self.handle.take() {
            tokio::runtime::Handle::current()
                .block_on(await_handle_bounded(handle, SHUTDOWN_TIMEOUT));
        }
    }
}
```

`JoinHandle`, `JoinError` (no longer referenced directly — the helper handles the `Err` arm),
`Duration`, `Instant`, `warn`/`error` are all already imported. If clippy flags `JoinError` as
now-unused, remove it from the line-23 import.

- [ ] **Step 5: Run the tests (pass)**

Run: `cargo test -p refbox await_handle_bounded`
Expected: PASS.

- [ ] **Step 6: Lint and commit** *(approval-gated)*

```bash
cargo clippy -p refbox -- -D warnings
git add refbox/src/sound_controller/mod.rs
git commit -m "fix(refbox): bound sound controller shutdown so a hang can't block restart"
```

---

## Task 5: Drift guard — exhaustively destructure `Cli` in `build_restart_argv`

**Why:** `build_restart_argv` reads `Cli` fields by name; a new flag added later would compile
while being silently omitted from restarts — re-introducing the exact bug PR #1073 fixed.
Destructuring exhaustively (no `..`) makes any future field break the build until its author
decides replay-or-exclude. Behaviour is unchanged, so the existing `restart_argv_tests` are the
regression proof.

**Files:**
- Modify: `refbox/src/main.rs` (`build_restart_argv`, lines 230-279)

- [ ] **Step 1: Confirm the existing behaviour tests pass (the safety net for this refactor)**

Run: `cargo test -p refbox restart_argv`
Expected: PASS (these stay green and prove behaviour is unchanged after the refactor).

- [ ] **Step 2: Refactor to an exhaustive destructure**

Replace the body of `build_restart_argv` (lines 230-279) with this version (same output, new guard
at the top):

```rust
pub(crate) fn build_restart_argv(args: &Cli) -> Vec<String> {
    // DRIFT GUARD: destructure every field with no `..`, so ANY future CLI field
    // added to `Cli` breaks the build here until its author consciously decides
    // whether it should be replayed across an in-app restart. This is what stops
    // the "restart drops CLI args" bug (PR #1073) from silently re-appearing.
    let Cli {
        no_simulate,
        verbose,
        scale,
        spacing,
        fullscreen,
        binary_port,
        json_port,
        serial_port,
        baud_rate,
        allow_http,
        all_events,
        log_location,
        log_max_file_size,
        num_old_logs,
        simulate_sunlight_display,
        // --- Deliberately NOT replayed (see this fn's doc comment) ---
        language: _,         // a restart is often triggered BY a language change
        is_simulator: _,     // this relaunches the MAIN app, never a sim child
        capture_previews: _, // dev-only; replaying it would exit immediately
    } = args;

    let mut argv: Vec<String> = Vec::new();

    if *no_simulate {
        argv.push("--no-simulate".to_string());
    }
    for _ in 0..*verbose {
        argv.push("--verbose".to_string());
    }
    argv.push("--scale".to_string());
    argv.push(scale.to_string());
    if let Some(spacing) = spacing {
        argv.push("--spacing".to_string());
        argv.push(spacing.to_string());
    }
    if *fullscreen {
        argv.push("--fullscreen".to_string());
    }
    argv.push("--binary-port".to_string());
    argv.push(binary_port.to_string());
    argv.push("--json-port".to_string());
    argv.push(json_port.to_string());
    if let Some(port) = serial_port {
        argv.push("--serial-port".to_string());
        argv.push(port.clone());
        argv.push("--baud-rate".to_string());
        argv.push(baud_rate.to_string());
    }
    if *allow_http {
        argv.push("--allow-http".to_string());
    }
    if *all_events {
        argv.push("--all-events".to_string());
    }
    if let Some(loc) = log_location {
        argv.push("--log-location".to_string());
        // A non-UTF-8 log path would already have panicked at startup before here.
        argv.push(loc.to_str().unwrap().to_string());
    }
    argv.push("--log-max-file-size".to_string());
    argv.push(log_max_file_size.to_string());
    argv.push("--num-old-logs".to_string());
    argv.push(num_old_logs.to_string());
    if *simulate_sunlight_display {
        argv.push("--simulate-sunlight-display".to_string());
    }

    argv
}
```

Keep the existing doc comment above the function (lines 219-229) intact.

- [ ] **Step 3: Verify behaviour unchanged and the guard compiles**

Run: `cargo test -p refbox restart_argv`
Expected: PASS — identical output to before; the destructure compiles (proving exhaustiveness).

- [ ] **Step 4: Lint and commit** *(approval-gated)*

```bash
cargo clippy -p refbox -- -D warnings
git add refbox/src/main.rs
git commit -m "fix(refbox): drift-guard build_restart_argv via exhaustive Cli destructure"
```

---

## Task 6: Full verification and hand-off to Pi validation

**Files:** none (verification only)

- [ ] **Step 1: Run the full project check**

Run: `just check`
Expected: PASS — fmt, clippy (`-D warnings`), tests, audit all clean.

- [ ] **Step 2: Confirm scope is clean (no files changed outside the three target files)**

Run: `git diff --name-only origin/master`
Expected: exactly `refbox/src/app/update_sender.rs`, `refbox/src/main.rs`,
`refbox/src/sound_controller/mod.rs` (the plan/spec docs must NOT appear).

- [ ] **Step 3: Record the Pi-validation gate (do not skip before publish)**

The branch is ready for PR + merge + release, but **must not be published as "latest" until the
🔧 PI checkpoints pass on the spare Pi**: restart via language change **five times in a row**, each
time confirming full-screen, scoreboard showing the clock, buzzer working, overlay reconnected,
logs unchanged. Triage: if the old window closes and no new window appears, suspect the sound
shutdown bound first; if the scoreboard stays dark, the serial retry predicate/budget needs
tuning; if the overlay can't connect, the TCP retry budget needs tuning.

- [ ] **Step 4: Code review + PR** *(approval-gated)*

Run `superpowers:requesting-code-review`, then open the PR (ask first) using the
`.claude/rules/pr-review.md` body format. Title: `fix(refbox): harden restart resilience
(serial/TCP retry, bounded sound shutdown, flag drift guard)`.

---

## Self-review — completed during planning

- **Spec coverage:** Phase 1 design §2.2 Change 1 → Tasks 2-3; Change 2 → Task 4; Change 3 →
  Task 5; §2.4 acceptance (5× restart) → Task 6 Step 3; §2.5 automated tests → Tasks 2-4 tests;
  §2.3 exclusions (explicit-exe-path refactor, updater) correctly absent. Covered.
- **Placeholder scan:** every code step has complete code; no TBD/TODO/"handle errors"; the only
  deferred-to-hardware items are explicitly marked `🔧 PI:` with concrete starting values.
- **Type consistency:** `is_transient_serial_error(&tokio_serial::Error)`,
  `open_one_serial_port_with_retry(SerialPortBuilder) -> Option<SerialStream>`,
  `is_transient_bind_error(&io::Error)`,
  `bind_with_retry((&str,u16), &str, Duration, Duration) -> Option<TcpListener>`,
  `await_handle_bounded(JoinHandle<()>, Duration)` — names/signatures consistent across tasks and
  match the confirmed master types (`handle: Option<JoinHandle<()>>`).
