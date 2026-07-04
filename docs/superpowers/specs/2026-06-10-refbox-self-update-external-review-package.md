# External Review Package — Refbox Restart & Self-Update

**Self-contained briefing for an independent reviewer (human or AI) with NO access to the
repository.** Everything needed to give a second opinion is included below, with the relevant code
quoted inline. Date: 2026-06-10.

---

## 1. What the software is

`refbox` is a Rust desktop GUI application (built with the `iced` 0.13 framework, `tokio` async
runtime) that runs underwater-hockey games at tournaments. It manages the game clock, scores, and
penalties, and drives poolside hardware:

- an **LED scoreboard** over a **serial** port,
- a **wireless referee button** over a **LoRa radio** (SPI),
- a **stream overlay** over **TCP** (ports 8000/8001),
- **audio/buzzer** output.

It is deployed on **Raspberry Pi 4/5** (`aarch64-unknown-linux-gnu`) as a **single binary**, and
also on Windows/Mac laptops. Releases are published on GitHub at **`AtlantisSports/uwh-refbox-rs`**
as a single combined zip of all platforms (currently a *draft* release, with **no** standalone Pi
binary and **no** checksum/signature asset).

## 2. The proposed feature (Raspberry-Pi only)

An operator-initiated in-app updater:

1. App Options page gets a **"Check Version"** button (disabled while a game is in progress) that
   opens an **Updates** page.
2. Updates page shows the current version and a yellow **"Check for Updates"** button. Status
   starts at **"Unknown"** each session.
3. On press, the app queries the latest GitHub release; if newer, the button becomes **"Install
   Update"** and status shows the new version.
4. On confirm: **download** the standalone aarch64 Pi binary → **verify a published checksum** →
   **smoke-test** that it launches → set current binary aside as a **one-deep backup** → **move the
   new binary into place** → **restart** via the app's existing restart mechanism.
5. A **"Revert to Previous Version"** button swaps the backup back.
6. Safety: feature disabled during a game; the manual SD-card update method remains as a fallback;
   the release pipeline will be changed (additively) to also publish the standalone Pi binary + a
   checksum file.

## 3. The EXISTING restart mechanism it builds on (SHIPPED but NEVER run at a real event)

After the `iced` run-loop returns, `main()` checks a global flag and respawns the executable:

```rust
// refbox/src/main.rs  (~line 507-522)
let result = iced::application(title, app::RefBoxApp::update, app::RefBoxApp::view)
    .subscription(app::RefBoxApp::subscription)
    .settings(settings)
    .window(window_settings)
    .style(app::RefBoxApp::application_style)
    .run_with(|| app::RefBoxApp::new(flags));

// If an in-app "Restart to Apply" path requested a restart, the iced
// runtime has just finished closing all windows. Spawn a fresh copy of
// the exe NOW (after the windows are gone) ...
if app::RESTART_PENDING.load(std::sync::atomic::Ordering::Relaxed) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
}
```

`RESTART_PENDING` is a process-global `AtomicBool` (refbox/src/app/mod.rs:78), set true by in-app
"restart to apply" paths (app-mode switch ~mod.rs:1010-1045, language/font change ~mod.rs:2686),
after persisting config and flushing a portal queue.

For contrast, the simulator child-spawn **does** reconstruct its arguments and detach stdio:

```rust
// refbox/src/main.rs  (~line 214-222)
pub(crate) fn spawn_sim_child(config: &SimSpawnConfig) -> std::io::Result<std::process::Child> {
    let bin_name = std::env::current_exe()?.into_os_string();
    let argv = build_sim_argv(config);
    Command::new(bin_name)
        .args(&argv)
        .stdin(Stdio::null())
        .spawn()
}
```

The binary accepts many `clap` command-line flags (refbox/src/main.rs:97-163) that materially
change behaviour, including: `--fullscreen` (kiosk display), `--serial-port <path>` (LED panel),
`--no-simulate` (real hardware vs simulator), `--binary-port`/`--json-port` (overlay TCP, default
8001/8000), `--baud-rate`, `--allow-http`, `--all-events`, `--log-location`, `--language`,
`--verbose`. `--fullscreen` is applied at startup (app/mod.rs ~1341).

## 4. Findings from our internal review (49 total; judged on merit)

**Confirmed by hand (BLOCKER):** the restart at main.rs:520 spawns the binary with **no
arguments**, so every restart relaunches with clap *defaults* — losing `--fullscreen`,
`--serial-port`, `--no-simulate`, log location, etc. On a tournament Pi this means a restart (from
a language/mode change today, or a self-update later) brings the app back **windowed, with no LED
panel, in simulation mode**. This affects the already-shipped restart paths.

**Other high-severity themes:**
- `spawn()` failure is silently discarded (`let _ = ...`) → a failed relaunch leaves **no running
  app and no log**.
- The relaunched process can race the old one for the **serial port and TCP ports**; the serial
  open uses `.unwrap()` → **panic** if the port is still held.
- **Linux binary replacement:** overwriting a running binary in place fails with **ETXTBSY**; the
  swap must be an atomic **rename** (write temp on same filesystem, then rename). The downloaded
  file needs the **executable bit** set. `std::env::current_exe()` resolves `/proc/self/exe`, which
  can read as a `(deleted)` path or a symlink/PATH location not equal to the install path — the
  swap must capture and target the real install path.
- Resource hand-off across restart depends on `iced` app state actually being **dropped** when
  `run()` returns; `SoundController::drop` does a **blocking** call inside `Drop` (untested
  ordering on hardware). The LoRa task/SPI + audio device re-acquisition on relaunch is
  unverified.
- **Game-gate** definition: `BetweenGames` includes half-time/break periods that are *within* a
  game; also a time-of-check/time-of-use gap exists between "Check" and "Install".
- **Download/verify:** GitHub REST API **rejects requests with no `User-Agent` header**. A plain
  checksum proves integrity, **not authenticity** (compromised release/account). Release assets
  **redirect** to `objects.githubusercontent.com` (don't leak auth across the redirect; keep
  https-only). No disk-space precondition. Anonymous API rate limit 60/hr/IP (multiple Pis behind
  one NAT).
- **Operability:** the app exposes/logs **no version string** (can't confirm an update took); there
  is **no safe headless self-test mode** (a naive smoke-launch opens a stray GUI window, spawns a
  sim child, and contends for TCP ports); no post-restart hardware-reconnection check.

## 5. Questions for you, the external reviewer

1. Is respawning via `Command::new(current_exe()).spawn()` (after `iced` returns) a sound restart
   strategy on Linux/aarch64, or is `execv`-style in-place replacement preferable? What are the
   trade-offs for a kiosk app holding a serial port, SPI radio, and TCP listeners?
2. Confirm the safest **atomic binary-swap** sequence on a Raspberry Pi SD card such that a power
   loss at *any* instant leaves a bootable binary at the install path. Validate the
   "write-temp-then-rename, capture path before swap, chmod +x, same-filesystem" approach.
3. After a rename-based swap, what does `std::env::current_exe()` return, and will the subsequent
   `spawn()` launch the new binary? Is capturing the path before the swap sufficient and necessary?
4. What is the most robust way to release the **serial port, SPI/LoRa, TCP ports, and audio device**
   from the old process before the new one acquires them, given `iced`'s shutdown and Rust `Drop`
   ordering (and a blocking call inside `SoundController::drop`)? Is a clean handshake possible, or
   is a brief retry/backoff on resource acquisition the pragmatic answer?
5. Is a **headless smoke-test** of a GUI binary feasible/worthwhile on a Pi (e.g. a hidden
   `--self-check` mode that initialises and exits without opening a window or grabbing hardware), or
   is checksum + (optional) signature sufficient pre-swap validation?
6. **Checksum vs signature:** given the threat model (a tournament Pi pulling from public GitHub
   releases), is a published SHA-256 acceptable, or is a minimal signature scheme warranted? If
   signing, what's the lightest defensible approach?
7. Any **brick scenario** we have not listed, and any safety net you'd add beyond: verify-before-
   swap, smoke-test, one-deep backup, in-app revert, game-disabled gate, and manual SD fallback?
8. Is "operator-initiated, disabled during a game, one-deep revert" the right control model for a
   safety-critical kiosk, or would you change the policy?
