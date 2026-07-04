# Refbox Self-Update Phase 2 (Updater Backend) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax. **All git operations stay with the orchestrator** (subagents edit + run cargo only — this repo has had subagents scramble branches). Commits are **approval-gated**. Work in the worktree from `origin/master`; use `cargo test -p refbox` / `cargo clippy -p refbox -- -D warnings` (bin crate: no `--lib`, no `--all-targets`).

**Goal:** Add the in-app "Check for Updates / Install / Revert" feature on the Raspberry Pi, wiring a real backend (GitHub check → download → SHA-256 verify → smoke-test → atomic swap → restart → best-effort auto-revert) behind the already-approved Updates-page UI preview.

**Architecture:** A new `refbox/src/updater/` module holds pure, unit-testable logic + thin async (reqwest/tokio) wrappers. The Updates page reuses the approved preview's `UpdateUiState` machine, driven by real async iced tasks (approach A) instead of the preview's simulated sleeps. The single relaunch routine gains an explicit-exe-path parameter so the updater can restart the swapped-in binary. Best-effort auto-revert via marker files in the config dir.

**Tech Stack:** Rust 2024 / MSRV 1.85; `reqwest` 0.12 (already a refbox dep); `sha2` (already in `Cargo.lock`); `tokio`; `clap`; `iced` 0.13; `confy`. Branches from `origin/master` (post-#1083).

**Design spec:** `docs/superpowers/specs/2026-06-15-refbox-self-update-phase2-design.md`. **Approved plan-mode plan:** `~/.claude/plans/snuggly-marinating-moon.md`.

---

## File structure

| File | Responsibility |
|------|----------------|
| `refbox/src/updater/mod.rs` | Module root; re-exports; the `Updater` orchestration types; `decide_on_startup`. |
| `refbox/src/updater/version.rs` | `Version` parse + `compare` (pure). |
| `refbox/src/updater/release.rs` | Parse GitHub latest-release JSON → `ReleaseInfo`; asset selection (pure). |
| `refbox/src/updater/verify.rs` | `verify_sha256` (pure). |
| `refbox/src/updater/swap.rs` | hard-link backup + rename swap + revert; backup naming (pure-ish, fs on temp dirs in tests). |
| `refbox/src/updater/marker.rs` | trial / rolled-back marker read/write/clear; `StartupDecision` (fs on temp dirs in tests). |
| `refbox/src/updater/net.rs` | async `check_latest` + `download_asset` (reqwest). |
| `refbox/src/main.rs` | `--self-check` flag + handler; relaunch refactor; startup auto-revert hook; pass `config_dir` to app. |
| `refbox/src/app/message.rs` | real `UpdateUiState`/`UpdateUiError` + `Updates*` messages. |
| `refbox/src/app/mod.rs` | `AppState::Updates`; real handlers; `view()` arm; game-gate predicate; trial-marker-clear on healthy. |
| `refbox/src/app/view_builders/configuration.rs` | `make_updates_page`; Check Version button + game gate. |
| `refbox/Cargo.toml` | add `sha2`. |
| `.github/workflows/release.yml` | publish standalone aarch64 binary + `.sha256`. |
| `refbox/translations/*` | new strings, all locales. |

---

## Task 1: Branch + green baseline

**Files:** none (git + verify; orchestrator-run).

- [ ] **Step 1 (orchestrator):** worktree from `origin/master` on branch `feat/refbox/self-update` (rename the EnterWorktree branch to this conventional name before any commit — the branch-name pre-commit hook rejects `worktree-…`).
- [ ] **Step 2:** `cargo test -p refbox` → PASS (post-#1083 baseline, ~243 tests).
- [ ] **Step 3:** confirm base: `grep -n "fn build_restart_argv" refbox/src/main.rs` → ~line 230; `grep -n "RESTART_PENDING" refbox/src/main.rs` → the relaunch block ~619.

---

## Task 2: `--self-check` CLI flag + handler + drift guard

**Files:** Modify `refbox/src/main.rs` (`Cli` ~98-167; `build_restart_argv` ~230-303; early-exit area ~411).

- [ ] **Step 1: Add the flag** to `Cli` after `simulate_sunlight_display`:
```rust
    #[clap(long, hide = true)]
    /// Probe that the binary starts on this machine, then exit 0. Used as the
    /// post-download smoke test before committing to a new binary.
    self_check: bool,
```
- [ ] **Step 2: Drift guard** — add to the exhaustive `Cli` destructure in `build_restart_argv`, in the "deliberately NOT replayed" group:
```rust
        self_check: _, // a smoke-test probe, never replayed into a real restart
```
- [ ] **Step 3: Handler** — immediately after the `capture_previews` early-return block (~line 414, before `is_simulator`):
```rust
    if args.self_check {
        // Smoke test: logging + config already initialised above. Prove we can
        // start on this machine without opening a window, spawning a sim child,
        // or grabbing hardware, then exit 0.
        info!("--self-check ok");
        return Ok(());
    }
```
- [ ] **Step 4: Test** (in the `restart_argv_tests` module): `--self-check` is never replayed.
```rust
    #[test]
    fn never_replays_self_check() {
        assert!(!argv_from(&["--self-check"]).contains(&"--self-check".to_string()));
    }
```
- [ ] **Step 5:** `cargo test -p refbox restart_argv` PASS; `cargo build -p refbox` then `./target/debug/refbox --self-check; echo $?` → prints `--self-check ok` and exits `0` (no window). `cargo fmt -p refbox` + `cargo clippy -p refbox -- -D warnings`.
- [ ] **Step 6 (orchestrator):** commit `feat(refbox): add hidden --self-check smoke-test flag`.

---

## Task 3: Relaunch refactor (explicit exe path)

**Files:** Modify `refbox/src/main.rs` (RESTART_PENDING block ~619-633).

- [ ] **Step 1:** Extract the spawn into a helper above `main()`:
```rust
/// Respawn the app: `exe` is the program file to launch, `argv` the replayed
/// start-up arguments. A failed spawn is logged (never silently swallowed).
fn respawn(exe: std::path::PathBuf, argv: &[String]) {
    info!("Restart requested: respawning {exe:?} with args {argv:?}");
    if let Err(e) = std::process::Command::new(exe)
        .args(argv)
        .stdin(Stdio::null())
        .spawn()
    {
        error!("Failed to respawn refbox on restart: {e}");
    }
}
```
- [ ] **Step 2:** Replace the RESTART_PENDING block body to call it with `current_exe()` (plain restart keeps current behaviour):
```rust
    if app::RESTART_PENDING.load(std::sync::atomic::Ordering::Relaxed) {
        match std::env::current_exe() {
            Ok(exe) => respawn(exe, &restart_argv),
            Err(e) => error!("Failed to locate current exe for restart respawn: {e}"),
        }
    }
```
- [ ] **Step 3:** `cargo test -p refbox` (behaviour unchanged), `cargo clippy -p refbox -- -D warnings`. (The updater will call `respawn(install_path, &restart_argv)` in Task 13.)
- [ ] **Step 4 (orchestrator):** commit `refactor(refbox): relaunch via explicit exe-path helper`.

---

## Task 4: `updater::version` — parse + compare

**Files:** Create `refbox/src/updater/mod.rs` (with `pub mod version;` + `mod` wiring) and `refbox/src/updater/version.rs`. Add `pub mod updater;` to `refbox/src/main.rs` module list.

- [ ] **Step 1: Failing tests** (`version.rs` `#[cfg(test)]`):
```rust
    #[test]
    fn parses_and_orders() {
        use std::cmp::Ordering::*;
        assert_eq!(Version::parse("0.4.2").unwrap(), Version { major: 0, minor: 4, patch: 2 });
        assert_eq!(Version::parse("v0.4.3").unwrap().cmp_to(&Version::parse("0.4.2").unwrap()), Greater);
        assert_eq!(Version::parse("0.4.2").unwrap().cmp_to(&Version::parse("0.4.2").unwrap()), Equal);
        assert_eq!(Version::parse("0.4.2").unwrap().cmp_to(&Version::parse("0.4.10").unwrap()), Less);
        assert!(Version::parse("garbage").is_none());
        assert!(Version::parse("0.4").is_none());
    }
```
- [ ] **Step 2:** Run → FAIL (undefined). 
- [ ] **Step 3: Implement** `version.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version { pub major: u32, pub minor: u32, pub patch: u32 }

impl Version {
    /// Parse `X.Y.Z` (a leading `v` is tolerated). Returns None on anything else.
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let mut it = s.split('.');
        let major = it.next()?.parse().ok()?;
        let minor = it.next()?.parse().ok()?;
        let patch = it.next()?.parse().ok()?;
        if it.next().is_some() { return None; }
        Some(Version { major, minor, patch })
    }
    pub fn cmp_to(&self, other: &Version) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
```
- [ ] **Step 4:** `cargo test -p refbox version` PASS; clippy clean.
- [ ] **Step 5 (orchestrator):** commit `feat(refbox): updater version parse/compare`.

---

## Task 5: `updater::release` — parse latest-release JSON + asset selection

**Files:** Create `refbox/src/updater/release.rs`; add `pub mod release;` to `updater/mod.rs`.

Asset naming matches Task 14's release: the standalone binary asset `refbox-aarch64-linux` and checksum `refbox-aarch64-linux.sha256`.

- [ ] **Step 1: Failing tests** with a captured minimal sample of GitHub's `/releases/latest` JSON (tag_name + assets[].name/browser_download_url). Cover: extracts version + both asset URLs; returns `Err` on a non-JSON body (captive portal); returns `Err` if the expected asset is missing.
```rust
    const SAMPLE: &str = r#"{"tag_name":"v0.4.3","assets":[
      {"name":"refbox-aarch64-linux","browser_download_url":"https://x/bin"},
      {"name":"refbox-aarch64-linux.sha256","browser_download_url":"https://x/sum"}]}"#;
    #[test]
    fn parses_release() {
        let r = ReleaseInfo::from_json(SAMPLE).unwrap();
        assert_eq!(r.version, Version::parse("0.4.3").unwrap());
        assert_eq!(r.binary_url, "https://x/bin");
        assert_eq!(r.checksum_url, "https://x/sum");
    }
    #[test]
    fn rejects_non_json() { assert!(ReleaseInfo::from_json("<html>login</html>").is_err()); }
    #[test]
    fn rejects_missing_asset() {
        assert!(ReleaseInfo::from_json(r#"{"tag_name":"v0.4.3","assets":[]}"#).is_err());
    }
```
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3: Implement** `ReleaseInfo { version: Version, binary_url: String, checksum_url: String }` with `from_json(&str) -> Result<ReleaseInfo, UpdateError>` using `serde_json::Value` (serde_json is already available via reqwest "json"/portal). Constants `BIN_ASSET = "refbox-aarch64-linux"`, `SUM_ASSET = "refbox-aarch64-linux.sha256"`. Define `UpdateError` enum here (or in `mod.rs`) with variants used across the module: `Network`, `NotJson`, `AssetMissing`, `BadVersion`, `Checksum`, `NoSpace`, `NotWritable`, `Io(String)` — each carrying enough for the UI error mapping in Task 9.
- [ ] **Step 4:** `cargo test -p refbox release` PASS; clippy clean.
- [ ] **Step 5 (orchestrator):** commit `feat(refbox): parse GitHub release + select Pi assets`.

---

## Task 6: `updater::verify` — SHA-256

**Files:** Create `refbox/src/updater/verify.rs`; add `pub mod verify;`. Modify `refbox/Cargo.toml` (add `sha2 = "0.10"` — already in `Cargo.lock`).

- [ ] **Step 1: Failing tests** (write bytes to a temp file; known SHA-256):
```rust
    #[test]
    fn verifies_good_and_rejects_tampered() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        std::fs::write(&p, b"hello").unwrap();
        // sha256("hello")
        let good = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_sha256(&p, good).unwrap());
        assert!(!verify_sha256(&p, "0000…").unwrap());
    }
```
(Use `tempfile` — confirm it's a dev-dependency; if not, add it under `[dev-dependencies]`, it's already in `Cargo.lock`.)
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3: Implement:**
```rust
use sha2::{Digest, Sha256};
/// Stream the file through SHA-256 and compare (case-insensitive hex) to `expected`.
pub fn verify_sha256(path: &std::path::Path, expected: &str) -> std::io::Result<bool> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut f, &mut hasher)?;
    let got = hasher.finalize();
    let got_hex = got.iter().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(got_hex.eq_ignore_ascii_case(expected.trim()))
}
```
- [ ] **Step 4:** `cargo test -p refbox verify` PASS; clippy clean.
- [ ] **Step 5 (orchestrator):** commit `feat(refbox): SHA-256 checksum verification`.

---

## Task 7: `updater::swap` — hard-link backup + rename + revert

**Files:** Create `refbox/src/updater/swap.rs`; add `pub mod swap;`.

- [ ] **Step 1: Failing tests** (temp dir simulating the install dir; "install path never empty" invariant; backup created; revert restores):
```rust
    #[test]
    fn swap_keeps_install_path_present_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path().join("refbox");
        std::fs::write(&install, b"OLD").unwrap();
        let newf = dir.path().join("refbox.new");
        std::fs::write(&newf, b"NEW").unwrap();
        let backup = swap_in_place(&install, &newf, &Version::parse("0.4.1").unwrap()).unwrap();
        assert_eq!(std::fs::read(&install).unwrap(), b"NEW");      // new in place
        assert_eq!(std::fs::read(&backup).unwrap(), b"OLD");        // backup is old
        assert!(backup.file_name().unwrap().to_str().unwrap().contains("0.4.1"));
        revert(&install, &backup).unwrap();
        assert_eq!(std::fs::read(&install).unwrap(), b"OLD");       // restored
    }
```
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3: Implement:** `swap_in_place(install: &Path, new: &Path, prev: &Version) -> io::Result<PathBuf>` = `let backup = install.parent().join(format!("refbox-v{prev}.bak"))`; remove a stale backup if present; `fs::hard_link(install, &backup)?`; `fs::rename(new, install)?`; return `backup`. `revert(install: &Path, backup: &Path) -> io::Result<()>` = `fs::rename(backup, install)`. Doc-comment the ordering (hard-link first so the install path is never empty) and that callers pass the **canonicalized** install path captured before any swap.
- [ ] **Step 4:** `cargo test -p refbox swap` PASS; clippy clean.
- [ ] **Step 5 (orchestrator):** commit `feat(refbox): atomic binary swap + revert`.

---

## Task 8: `updater::marker` — trial/rolled-back markers + startup decision

**Files:** Create `refbox/src/updater/marker.rs`; add `pub mod marker;`. Mirror the atomic-write pattern in `refbox/src/portal_manager/queue.rs`.

- [ ] **Step 1: Failing tests** (temp dir as config dir):
```rust
    #[test]
    fn trial_then_healthy_is_normal() {
        let d = tempfile::tempdir().unwrap();
        write_trial(d.path(), &v("0.4.2"), &v("0.4.1")).unwrap();
        clear_trial(d.path()).unwrap();
        assert!(matches!(decide_on_startup(d.path()), StartupDecision::Normal));
    }
    #[test]
    fn uncleared_trial_triggers_autorevert() {
        let d = tempfile::tempdir().unwrap();
        write_trial(d.path(), &v("0.4.2"), &v("0.4.1")).unwrap();
        match decide_on_startup(d.path()) {
            StartupDecision::AutoRevert { backup_version } => assert_eq!(backup_version, v("0.4.1")),
            other => panic!("expected AutoRevert, got {other:?}"),
        }
    }
    #[test]
    fn rolled_back_marker_shows_message_once() {
        let d = tempfile::tempdir().unwrap();
        write_rolled_back(d.path()).unwrap();
        assert!(matches!(decide_on_startup(d.path()), StartupDecision::ShowRolledBack));
        clear_rolled_back(d.path()).unwrap();
        assert!(matches!(decide_on_startup(d.path()), StartupDecision::Normal));
    }
```
(`v(s)` = test helper `Version::parse(s).unwrap()`.)
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3: Implement:** constants `TRIAL = "update_trial.marker"`, `ROLLED_BACK = "update_rolled_back.marker"`. Trial file stores `trying\nbackup` versions (two lines). `write_trial`/`clear_trial`/`write_rolled_back`/`clear_rolled_back` (atomic tmp-write + rename, `fsync` the dir/file before rename per finding F). `decide_on_startup(dir) -> StartupDecision` where `enum StartupDecision { Normal, AutoRevert { backup_version: Version }, ShowRolledBack }` — precedence: an uncleared **trial** marker ⇒ `AutoRevert`; else a **rolled-back** marker ⇒ `ShowRolledBack`; else `Normal`. `#[derive(Debug)]` the enum.
- [ ] **Step 4:** `cargo test -p refbox marker` PASS; clippy clean.
- [ ] **Step 5 (orchestrator):** commit `feat(refbox): auto-revert markers + startup decision`.

---

## Task 9: `updater::net` — async check + download

**Files:** Create `refbox/src/updater/net.rs`; add `pub mod net;`.

This task has no unit test (network); it's covered by the on-Pi gate. Keep it thin — all parsing/decisions live in the tested pure functions.

- [ ] **Step 1: Implement `check_latest`:**
```rust
const REPO: &str = "AtlantisSports/uwh-refbox-rs";
const UA: &str = concat!("uwh-refbox-rs/", env!("CARGO_PKG_VERSION"));

/// Query GitHub's latest published release (drafts/pre-releases excluded by the API).
pub async fn check_latest() -> Result<ReleaseInfo, UpdateError> {
    let client = reqwest::ClientBuilder::new()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(UA)
        .build()
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    let resp = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send().await.map_err(|_| UpdateError::Network)?;
    if resp.status() == reqwest::StatusCode::FORBIDDEN { return Err(UpdateError::RateLimited); }
    if !resp.status().is_success() { return Err(UpdateError::Network); }
    let body = resp.text().await.map_err(|_| UpdateError::Network)?;
    ReleaseInfo::from_json(&body) // maps non-JSON (captive portal) → NotJson
}
```
- [ ] **Step 2: Implement `download_asset`** — a **separate client with no auth header** (leak-safe across the objects.githubusercontent.com redirect); stream to a temp file in the install dir; also fetch the checksum text. Pre-flight: check the install dir is writable and (best-effort) free space, mapping to `NotWritable`/`NoSpace`.
```rust
pub async fn download_to(url: &str, dest: &std::path::Path) -> Result<(), UpdateError> {
    let client = reqwest::ClientBuilder::new()
        .https_only(true).timeout(std::time::Duration::from_secs(180))
        .user_agent(UA).build().map_err(|e| UpdateError::Io(e.to_string()))?;
    let resp = client.get(url).send().await.map_err(|_| UpdateError::Network)?;
    if !resp.status().is_success() { return Err(UpdateError::Network); }
    let bytes = resp.bytes().await.map_err(|_| UpdateError::Network)?;
    std::fs::write(dest, &bytes).map_err(|e| UpdateError::Io(e.to_string()))
}
pub async fn fetch_text(url: &str) -> Result<String, UpdateError> { /* same client; resp.text() */ }
```
- [ ] **Step 3:** `cargo build -p refbox`; `cargo clippy -p refbox -- -D warnings`.
- [ ] **Step 4 (orchestrator):** commit `feat(refbox): async GitHub check + asset download`.

---

## Task 10: UI types + Updates page (mirror the approved preview)

**Files:** Modify `refbox/src/app/message.rs`, `refbox/src/app/mod.rs`, `refbox/src/app/view_builders/configuration.rs`. **Mirror commit `890a6562` element-for-element, minus the scenario picker.** (`git show 890a6562 -- <file>` is the reference.)

- [ ] **Step 1: message.rs** — add the real types (drop `UpdateScenario`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateUiError { NoInternet, BadDownload }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateUiState {
    Unknown, Checking, UpToDate, UpdateAvailable, ConfirmInstall,
    Downloading, Verifying, Installing, Restarting, RevertConfirm,
    Error(UpdateUiError),
}
```
and the `Message` variants from the preview **minus `UpdatesSetScenario`**: `OpenUpdatesPage, UpdatesCheck, UpdatesInstall, UpdatesConfirmInstall, UpdatesRevert, UpdatesConfirmRevert, UpdatesStep(UpdateUiState), UpdatesBack`. Wire them into the three `Message` impls exactly as `890a6562` does (the `is_*`/`PartialEq`/catch-all arms) but without the scenario variant.
- [ ] **Step 2: mod.rs** — add `AppState::Updates { state: UpdateUiState, backup_available: bool }` (drop `scenario`); add the `view()` dispatch arm calling `make_updates_page(data, state, backup_available)`.
- [ ] **Step 3: configuration.rs** — add `make_updates_page<'a>(data, state: &UpdateUiState, backup_available: bool)` mirroring the preview's function at `890a6562` configuration.rs:1941 **but delete the `scenario_block`** (preview lines ~2069-2095) and the `scenario` param. Keep: the time ribbon, the version+primary-action row, the status row, the blank/revert row, the footer (Back/Cancel, disabled while `Restarting`). Use existing `make_button`/`make_value_button`; styles `yellow_button`/`light_gray_button`/`red_button` as in the preview.
- [ ] **Step 4:** `cargo build -p refbox`; `cargo clippy -p refbox -- -D warnings`. (No behavior test yet — handlers are stubbed/temporary until Task 12; for now `OpenUpdatesPage` may set `AppState::Updates { Unknown, false }` and other `Updates*` messages can be minimally handled to compile.)
- [ ] **Step 5 (orchestrator):** commit `feat(refbox): updates page UI ported from preview (no backend yet)`.

---

## Task 11: Check Version button + game gate

**Files:** Modify `refbox/src/app/view_builders/configuration.rs` (`make_cancel_apply_footer` + its ~5 call sites: App 951, Display 1131, Sound 1276, Remotes 1395, Game/Language) and the caller that knows the snapshot.

- [ ] **Step 1:** Add `game_in_progress: bool` param to `make_cancel_apply_footer`. For `ConfigPage::App`, render the blue **"Check Version"** button between cancel/apply; set `.on_press(Message::OpenUpdatesPage)` only when `!game_in_progress` (otherwise no `on_press` → greyed/disabled, per `[[reference_iced_button_no_onpress_disabled]]`). Other pages: unchanged footer.
- [ ] **Step 2:** Thread `game_in_progress` from the view: compute `self.snapshot.current_period != GamePeriod::BetweenGames` where `make_app_config_page` is built, and pass it down. Update all `make_cancel_apply_footer` call sites to pass the bool (false for non-App pages is fine).
- [ ] **Step 3:** `cargo build -p refbox`; clippy clean. Manual: button present on App Options, greyed during a game, active between games.
- [ ] **Step 4 (orchestrator):** commit `feat(refbox): Check Version button gated on game state`.

---

## Task 12: Wire the real backend to the UI (approach A)

**Files:** Modify `refbox/src/app/mod.rs` (the `Updates*` handlers). Replace the preview's simulated `UpdatesStep` sleep-driver with real async `Task::perform` calls into `updater::net`/`swap`/`verify`, advancing `UpdateUiState`. Add result-carrying messages as needed (e.g. extend `UpdatesStep`/add `UpdatesCheckDone(Result<…>)`) — keep them in message.rs with the same wiring discipline.

- [ ] **Step 1: `UpdatesCheck`** → state `Checking`, `Task::perform(updater::net::check_latest(), …)`; on result compare to `env!("CARGO_PKG_VERSION")` → `UpToDate` or `UpdateAvailable` (store the `ReleaseInfo`), or `Error(NoInternet)`.
- [ ] **Step 2: `UpdatesInstall`** → `ConfirmInstall`. **`UpdatesConfirmInstall`** → re-check the game gate (abort to `Error`/back if a game started); then the real pipeline: `Downloading` (download binary+checksum to temp in install dir) → `Verifying` (`verify_sha256`; mismatch → `Error(BadDownload)`, nothing swapped) → `Installing` (smoke-test: run the temp binary `--self-check` + the **captured restart argv** via `std::process::Command`, check exit 0; then `swap_in_place` with the canonicalized install path captured **before** the swap; `marker::write_trial`) → `Restarting` (set `RESTART_PENDING`, request window close so `main` calls `respawn(install_path, &restart_argv)`).
- [ ] **Step 3: `UpdatesConfirmRevert`** → `Restarting`; `marker`/`swap::revert` to the `.bak`; restart.
- [ ] **Step 4: `backup_available`** — compute from a real `refbox-v*.bak` present next to the binary (not a preview bool).
- [ ] **Step 5: `UpdatesBack`** — same navigation as the preview (idle → `EditGameConfig(ConfigPage::App)`; progress/confirm → previous state; disabled while `Restarting`).
- [ ] **Step 6:** Remove ALL remaining `// PREVIEW` scaffolding (`UpdateScenario`, `UpdatesSetScenario`, simulated sleeps, scenario picker) repo-wide: `grep -rn "PREVIEW\|UpdateScenario\|UpdatesSetScenario" refbox/src` → empty.
- [ ] **Step 7:** `cargo test -p refbox`; `cargo clippy -p refbox -- -D warnings`; `cargo fmt`.
- [ ] **Step 8 (orchestrator):** commit `feat(refbox): wire real updater backend to Updates page`.

---

## Task 13: Startup auto-revert hook + healthy-clear

**Files:** Modify `refbox/src/main.rs` (after `config_dir` ~555, before launching the UI) and `refbox/src/app/mod.rs` (clear the trial marker on healthy).

- [ ] **Step 1: main.rs** — after `config_dir` is known, `match updater::marker::decide_on_startup(&config_dir)`:
  - `AutoRevert { backup_version }` → `swap::revert` (backup path from `config-dir`-adjacent install path — actually the install dir = exe dir; capture `current_exe()` dir), `marker::clear_trial`, `marker::write_rolled_back`, then `respawn(current_exe()?, &restart_argv)` and `return Ok(())` (don't open a window — the relaunch will).
  - `ShowRolledBack` → pass a flag into `RefBoxAppFlags` so the app starts on `AppState::Updates { Unknown, backup_available }` with the rolled-back message, and `marker::clear_rolled_back` (clear immediately so it's strictly once).
  - `Normal` → unchanged.
- [ ] **Step 2: app/mod.rs** — when the app reaches a healthy running state (first reach of `AppState::MainPage` after launch, after a short settle ~20s), call `marker::clear_trial(&config_dir)`. Use a simple one-shot: on startup arm a 20s `Task` that, if still running and on the main page, clears the trial marker. (Pass `config_dir` into the app via `RefBoxAppFlags`.)
- [ ] **Step 3:** `cargo test -p refbox`; clippy clean. (Behavioural validation is the on-Pi gate.)
- [ ] **Step 4 (orchestrator):** commit `feat(refbox): startup auto-revert + healthy-state marker clear`.

---

## Task 14: release.yml — publish standalone binary + checksum

**Files:** Modify `.github/workflows/release.yml`.

- [ ] **Step 1:** In `build-rpi`, after the existing `upload-artifact` step:
```yaml
    - name: Compute SHA-256 checksum
      run: |
        sha256sum target/aarch64-unknown-linux-gnu/release/refbox | awk '{print $1}' \
          > target/aarch64-unknown-linux-gnu/release/refbox.sha256
    - uses: actions/upload-artifact@v4
      with:
        name: refbox-rpi-sha256
        path: target/aarch64-unknown-linux-gnu/release/refbox.sha256
```
- [ ] **Step 2:** In `upload-release`, add a download for `refbox-rpi-sha256` (to `release/rpi-sha256`), and **after** the `zip -r ../refbox.zip .` step, stage the standalone assets so they're NOT inside the zip:
```yaml
    - name: Stage standalone Pi assets
      run: |
        cp "release/Raspberry Pi/refbox" refbox-aarch64-linux
        cp release/rpi-sha256/refbox.sha256 refbox-aarch64-linux.sha256
```
and extend the `softprops/action-gh-release` `files:` to:
```yaml
        files: |
          refbox.zip
          refbox-aarch64-linux
          refbox-aarch64-linux.sha256
```
(keep `draft: true`, `generate_release_notes: true`). Asset names match Task 5's `BIN_ASSET`/`SUM_ASSET`.
- [ ] **Step 3:** Validate YAML (`just`/`yamllint` if available, else careful review). Cannot run the release workflow locally — exercised by the on-Pi test release.
- [ ] **Step 4 (orchestrator):** commit `chore(ci): publish standalone Pi binary + checksum`.

---

## Task 15: Translations

**Files:** `refbox/translations/*` (every locale).

- [ ] **Step 1:** Add Fluent keys for every operator-facing string: "Check Version", "Current version", "Check for Updates", "Install Update", "Continue", "Up to date.", "Update available: {$version}", "Checking…", "Downloading…", "Checking the download…", "Installing…", "Restarting…", "Revert to Previous Version ({$version})", "Back", "Cancel", the rollback message, and the error strings (no internet / bad download / busy / no space / not writable). Replace the preview's literal strings in `make_updates_page` with `fl!()` calls.
- [ ] **Step 2:** Best-guess translation in ALL locales — no English placeholders ([[feedback_translate_all_locales_no_placeholders]]). Diff each locale against `en-US` to confirm parity.
- [ ] **Step 3:** `cargo test -p refbox`; clippy clean; `cargo build` (Fluent keys resolve).
- [ ] **Step 4 (orchestrator):** commit `feat(refbox): translate self-update UI strings (all locales)`.

---

## Task 16: Full verification + review + PR

**Files:** none (verify + review).

- [ ] **Step 1:** `just check` → exit 0 (fmt, clippy, tests, audit clean).
- [ ] **Step 2:** Scope check: `git diff --name-only origin/master` only the files in the table.
- [ ] **Step 3:** Final independent code review (`superpowers:requesting-code-review` or a review subagent) over the whole diff — focus on the swap/revert correctness, the auth-leak-safe download, the marker one-time semantics, and the game gate.
- [ ] **Step 4 (orchestrator):** push + open PR (approval-gated), body per `.claude/rules/pr-review.md`, **stating the on-Pi Tests B–E gate before publish**.
- [ ] **Step 5:** Record the on-Pi validation plan (throwaway v0.4.3 release; Tests B–E + deliberate-bad-build auto-revert; delete after). NOT done in CI.

---

## Self-review (completed during planning)

- **Spec coverage:** §3 flow → Tasks 5/6/7/9/12; §4 safety/game-gate → Tasks 11/12; §5 auto-revert → Tasks 8/13; §6 architecture → Tasks 2/3/4-9/10-12/14; §7 testing → per-task unit tests + Task 16 on-Pi; §8 translations → Task 15. Covered.
- **Placeholders:** pure-logic tasks have complete code + tests; integration/UI tasks specify exact functions/signatures and the `890a6562` mirror source; the only non-code item is the on-Pi gate (hardware, by design).
- **Type consistency:** `Version`, `ReleaseInfo{version,binary_url,checksum_url}`, `UpdateError`, `verify_sha256(&Path,&str)->io::Result<bool>`, `swap_in_place(&Path,&Path,&Version)->io::Result<PathBuf>`, `revert(&Path,&Path)`, `StartupDecision{Normal,AutoRevert{backup_version},ShowRolledBack}`, `respawn(PathBuf,&[String])`, `UpdateUiState`/`UpdateUiError`, `AppState::Updates{state,backup_available}` — consistent across tasks; asset names shared between Task 5 and Task 14.
