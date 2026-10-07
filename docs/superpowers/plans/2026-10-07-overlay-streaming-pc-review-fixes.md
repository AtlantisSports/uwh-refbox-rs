# Overlay streaming-PC fixes — review follow-ups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix every finding from the 2026-10-07 code review of branch `feat/overlay/streaming-pc-setup`, and make NDI's installer run only when the operator clicks **Install NDI** in the overlay's preview window.

**Architecture:** `overlay/src/ndi_runtime.rs` is split into pure, cross-platform decision functions (tested on every platform by `just check`) and thin Windows glue (download, PowerShell, elevated launch, DLL preload), compile-checked from Linux with the installed `x86_64-pc-windows-gnu` target. The bridge-port move from 8099 to 8098 becomes a one-time migration, recorded in each program's settings file, so a deliberate 8099 chosen later is respected.

**Tech Stack:** Rust 2024 (MSRV 1.85), macroquad 0.4, reqwest 0.12.23, serde/serde_json, confy; Windows PowerShell 5.1 (`Get-AuthenticodeSignature`, `Start-Process -Verb RunAs`), kernel32 `LoadLibraryExW`.

**Spec:** the review findings `overlay-code-review-findings.md` (copied into this plan's Findings section below) plus the PO decisions of 2026-10-07 (below). No ADR covers the overlay installer.

## Findings (from the code review, 2026-10-07)

1. `ndi_runtime.rs:256`: the signer check is `subject.contains("Vizrt")`. Any trusted certificate with "Vizrt" anywhere passes, any Vizrt-signed exe passes, and redirects aren't limited to HTTPS.
2. `ndi_runtime.rs:225`: a fixed, predictable name in `%TEMP%`; the file is checked by path and then launched elevated by path (time-of-check/time-of-use); two overlays collide.
3. `main.rs:402`: a `/DELAYLOAD` failure is a structured exception, so a broken engine crashes the overlay instead of "continue without NDI"; `find_engine` only checks that the file exists.
4. `ndi_runtime.rs:221`: `reqwest::get` has no timeout or size limit and buffers the whole body, so it can hang forever.
5. `overlay-bridge/config.rs:222`: the 8099 migration runs on every start; a deliberately typed `--port 8099` is saved, then discarded on the next start.
6. `overlay/main.rs:261`: the overlay's migration matches only exactly `http://127.0.0.1:8099`; `localhost:8099` or a bridge on another PC at `:8099` is left on a dead port.
7. `ndi_runtime.rs:95`: a process-wide `set_current_dir` for the DLL search is racy and weakens the search path. Folded into 3.
8. `ndi_runtime.rs:231`: `ps_quote` misses curly quotes; `powershell` and `reg` are run by bare name.
9. Automatic download plus UAC prompt on every start without NDI, repeated after a decline. **PO decision below.**
10. `discovery.rs:917`: lost the only full 254-address scan test; its assertion message is stale.

## PO decisions (2026-10-07)

- Fix all findings on this branch before its PR goes up.
- **The NDI install happens only when the operator asks:** the overlay's preview window says NDI isn't installed and shows an **Install NDI** button. Nothing pops up unless the operator clicks it.

## Facts checked for this plan (2026-10-07)

- `https://ndi.link/NDIRedistV6` redirects over HTTPS to `https://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20Runtime.exe` (9,648,232 bytes).
- Its Authenticode signer: `CN = Vizrt AG, O = Vizrt AG, L = Zürich, C = CH`, issued by `CN = DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1` (`O = "DigiCert, Inc."`).
- Its version info: ProductName `NDI 6 Runtime`, FileDescription `NDI 6 Runtime Setup`, CompanyName `NDI`.
- `cargo check -p overlay --features bridge --target x86_64-pc-windows-gnu` works on this machine (11 existing warnings, none in `ndi_runtime.rs`).
- Windows DLL search: "If a DLL with the same module name is already loaded in memory, the system ... resolves to the loaded DLL, no matter which directory it is in" (Microsoft, *Dynamic-link library search order*). So a DLL preloaded by full path satisfies the later delay-load of `Processing.NDI.Lib.x64.dll` by name.

## Global Constraints

- No new crates and no new features on existing crates.
- Pure decision logic in `ndi_runtime.rs` is NOT behind `cfg(windows)`, so `just check` on Linux runs its tests. Only OS glue is `#[cfg(windows)]`.
- `overlay/src/main.rs` declares the module as `#[cfg(any(feature = "ndi", test))] mod ndi_runtime;`, and the module starts with `#![cfg_attr(not(feature = "ndi"), allow(dead_code))]`.
- Gates for every task: `just check` passes, AND `cargo check -p overlay --features bridge --target x86_64-pc-windows-gnu --tests` passes with no warning located in `overlay/src/ndi_runtime.rs`.
- Every `unsafe` block carries a `// SAFETY:` comment.
- Nothing NDI-related is ever drawn into the NDI picture. Notes and the button go on the local preview only, after the canvas is drawn to the screen, as today.
- Exact strings (use verbatim):
  - `MANUAL_HELP` = `Click Install NDI to try again, or install the NDI engine from https://ndi.link/NDIRedistV6 (or NDI Tools from https://ndi.video/tools/) and restart the overlay.`
  - Missing note = `NDI off: NDI isn't installed on this PC. Click Install NDI (Windows will ask for permission).`
  - Button label = `Install NDI`
- Leave changes uncommitted (the controller commits after review).

## Review Focus

1. The operator double-clicks **Install NDI**: exactly one installer runs (Task 2 test: a second `begin_install` while installing returns false).
2. The operator says No at the Windows prompt: the overlay keeps running, shows why, offers the button again, and never prompts by itself (Task 2 tests: `install_allowed` after a failure; `start` never installs).
3. The download is redirected to plain HTTP or another host: refused, with the manual help shown (Task 1 test: `redirect_allowed`).
4. The venue network stalls mid-download, or the server sends something huge: a clear error within the time limits, never a hang or memory blow-up (Task 1 test: `within_size_limit`; time limits on the client).
5. A settings file where the operator deliberately chose 8099 after the one-time move: kept (Task 3 tests, both programs).

---

### Task 1: The installer is downloaded, checked and started safely (findings 1, 2, 4, 8)

**Files:**
- Modify: `overlay/src/ndi_runtime.rs`
- Modify: `overlay/src/main.rs` (module declaration line only)

**Interfaces:**
- Produces (pure, all platforms):
  - `pub struct InstallerFacts { pub status: String, pub signer_name: String, pub signer_subject: Vec<String>, pub issuer_name: String, pub product_name: String }` with `#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]`
  - `pub fn installer_is_acceptable(facts: &InstallerFacts) -> Result<(), String>`
  - `pub fn redirect_allowed(url: &reqwest::Url) -> bool`
  - `pub fn within_size_limit(so_far: u64, more: u64) -> Result<u64, String>`
- Produces (Windows): `fn install_engine(progress: &dyn Fn(&str)) -> Result<(), String>`. It downloads, checks and runs the installer, and does NOT look for the engine afterwards (Task 2 does that).

- [ ] **Step 1: Make the module compile for tests everywhere.** In `overlay/src/main.rs` change `#[cfg(feature = "ndi")] mod ndi_runtime;` to `#[cfg(any(feature = "ndi", test))] mod ndi_runtime;`. Add `#![cfg_attr(not(feature = "ndi"), allow(dead_code))]` as the first line of `ndi_runtime.rs`. Change the test module gate from `#[cfg(all(test, windows))]` to `#[cfg(test)]`, and keep `parse_reg_query`'s test under `#[cfg(windows)]` inside it (that function stays Windows-only).

- [ ] **Step 2: Write the failing tests** (in `ndi_runtime.rs`'s `mod tests`):

```rust
use super::*;

fn genuine() -> InstallerFacts {
    InstallerFacts {
        status: "Valid".into(),
        signer_name: "Vizrt AG".into(),
        signer_subject: vec![
            "CN=Vizrt AG".into(),
            "O=Vizrt AG".into(),
            "L=Zürich".into(),
            "C=CH".into(),
        ],
        issuer_name: "DigiCert Trusted G4 Code Signing RSA4096 SHA384 2021 CA1".into(),
        product_name: "NDI 6 Runtime".into(),
    }
}

#[test]
fn only_ndis_runtime_installer_signed_by_vizrt_is_accepted() {
    assert_eq!(installer_is_acceptable(&genuine()), Ok(()));

    let not_valid = InstallerFacts { status: "HashMismatch".into(), ..genuine() };
    assert!(installer_is_acceptable(&not_valid).is_err());

    // "Vizrt" somewhere in another company's name is not Vizrt.
    let lookalike = InstallerFacts {
        signer_name: "Vizrtx Ltd".into(),
        signer_subject: vec!["CN=Vizrtx Ltd".into(), "O=Vizrtx Ltd".into()],
        ..genuine()
    };
    assert!(installer_is_acceptable(&lookalike).is_err());

    // The right common name but another organisation.
    let wrong_org = InstallerFacts {
        signer_subject: vec!["CN=Vizrt AG".into(), "O=Someone Else".into()],
        ..genuine()
    };
    assert!(installer_is_acceptable(&wrong_org).is_err());

    // Any other Vizrt program is not NDI's runtime installer.
    let other_product = InstallerFacts { product_name: "Vizrt Viz Engine".into(), ..genuine() };
    assert!(installer_is_acceptable(&other_product).is_err());

    // A certificate not issued by DigiCert.
    let other_issuer = InstallerFacts { issuer_name: "Some Test CA".into(), ..genuine() };
    assert!(installer_is_acceptable(&other_issuer).is_err());
}

#[test]
fn the_download_only_follows_https_links_to_ndis_own_sites() {
    let ok = |u: &str| redirect_allowed(&reqwest::Url::parse(u).unwrap());
    assert!(ok("https://ndi.link/NDIRedistV6"));
    assert!(ok("https://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20Runtime.exe"));
    assert!(!ok("http://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20Runtime.exe"));
    assert!(!ok("https://downloads.ndi.tv.example.com/x.exe"));
    assert!(!ok("https://example.com/NDI%206%20Runtime.exe"));
}

#[test]
fn a_download_larger_than_the_limit_is_refused() {
    assert_eq!(within_size_limit(0, 9_648_232), Ok(9_648_232));
    assert!(within_size_limit(MAX_INSTALLER_BYTES - 1, 1).is_ok());
    assert!(within_size_limit(MAX_INSTALLER_BYTES, 1).is_err());
    assert!(within_size_limit(u64::MAX, 1).is_err());
}
```

- [ ] **Step 3: Run them to see them fail.** Run `cargo test -p overlay ndi_runtime`. Expected: compile errors (`InstallerFacts`, `installer_is_acceptable`, `redirect_allowed`, `within_size_limit`, `MAX_INSTALLER_BYTES` not found).

- [ ] **Step 4: Write the pure functions** (outside `mod windows`), replacing `EXPECTED_SIGNER` and `signature_is_acceptable`:

```rust
/// The organisation NDI's runtime installer is signed by (checked 2026-10-07 against
/// https://ndi.link/NDIRedistV6: `CN=Vizrt AG, O=Vizrt AG, L=Zürich, C=CH`).
pub const EXPECTED_SIGNER: &str = "Vizrt AG";
/// The certificate authority that issued that certificate (DigiCert Trusted G4 Code Signing…).
const EXPECTED_ISSUER_PREFIX: &str = "DigiCert ";
/// The installer's own product name, so another program Vizrt signed isn't run instead.
const EXPECTED_PRODUCT: &str = "NDI 6 Runtime";
/// The real installer is about 10 MB.
pub const MAX_INSTALLER_BYTES: u64 = 100 * 1024 * 1024;
/// The only places the download may come from (NDI's link service and its download server).
const ALLOWED_HOSTS: [&str; 2] = ["ndi.link", "downloads.ndi.tv"];

/// What Windows reports about a downloaded installer: gathered by PowerShell as JSON (see
/// `windows::installer_facts`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct InstallerFacts {
    /// `Get-AuthenticodeSignature`'s status, `Valid` when Windows trusts the signature.
    pub status: String,
    /// The signing certificate's common name.
    pub signer_name: String,
    /// The signing certificate's subject, one `KEY=value` part per entry.
    pub signer_subject: Vec<String>,
    /// The common name of the authority that issued the signing certificate.
    pub issuer_name: String,
    /// The file's ProductName.
    pub product_name: String,
}

/// Accepts the file only if Windows trusts its signature, it is signed by Vizrt AG (both the
/// common name and the organisation must match exactly) through DigiCert, and it calls itself
/// NDI's runtime installer.
pub fn installer_is_acceptable(facts: &InstallerFacts) -> Result<(), String> {
    if facts.status != "Valid" {
        return Err(format!("its signature status is \"{}\"", facts.status));
    }
    let organisation = format!("O={EXPECTED_SIGNER}");
    if facts.signer_name != EXPECTED_SIGNER
        || !facts.signer_subject.iter().any(|part| part.trim() == organisation)
    {
        return Err(format!(
            "it is signed by \"{}\", not {EXPECTED_SIGNER}",
            facts.signer_name
        ));
    }
    if !facts.issuer_name.starts_with(EXPECTED_ISSUER_PREFIX) {
        return Err(format!(
            "its certificate was issued by \"{}\", not DigiCert",
            facts.issuer_name
        ));
    }
    if facts.product_name.trim() != EXPECTED_PRODUCT {
        return Err(format!(
            "it is \"{}\", not NDI's runtime installer",
            facts.product_name.trim()
        ));
    }
    Ok(())
}

/// Only HTTPS links to NDI's own sites are followed.
pub fn redirect_allowed(url: &reqwest::Url) -> bool {
    url.scheme() == "https" && url.host_str().is_some_and(|host| ALLOWED_HOSTS.contains(&host))
}

/// Adds `more` bytes to the `so_far` already downloaded, refusing anything past
/// [`MAX_INSTALLER_BYTES`].
pub fn within_size_limit(so_far: u64, more: u64) -> Result<u64, String> {
    so_far
        .checked_add(more)
        .filter(|total| *total <= MAX_INSTALLER_BYTES)
        .ok_or_else(|| "the download is far larger than NDI's installer".to_string())
}
```

- [ ] **Step 5: Run the tests.** Run `cargo test -p overlay ndi_runtime`. Expected: PASS.

- [ ] **Step 6: Rewrite the Windows glue in `mod windows`** so that it does all of the following (code below):
  - (a) Downloads with a client that has a time limit at every stage and follows only allowed redirects.
  - (b) Streams the body with the size limit.
  - (c) Writes into a brand-new folder of its own.
  - (d) Re-opens the file read-only with write and delete sharing denied, and keeps that handle open across the check and the run, so nobody can swap the file in between.
  - (e) Gives PowerShell the path through an environment variable, never inside the script text.
  - (f) Runs PowerShell and `reg` by full System32 path.

```rust
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Windows `FILE_SHARE_READ`: others may read the file, nobody may write or delete it.
const FILE_SHARE_READ: u32 = 0x0000_0001;
/// The environment variable PowerShell reads the installer's path from.
const PATH_VARIABLE: &str = "UWH_NDI_INSTALLER";

/// `%SystemRoot%\System32\<rest>`, so a same-named program elsewhere on the PATH is never run.
fn system32(rest: &str) -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    PathBuf::from(root).join("System32").join(rest)
}

fn powershell() -> Command {
    let mut command = Command::new(system32(r"WindowsPowerShell\v1.0\powershell.exe"));
    command.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"]);
    command
}

/// Downloads, checks and runs NDI's runtime installer. Windows asks for permission; the person
/// accepts NDI's licence in the installer.
pub fn install_engine(progress: &dyn Fn(&str)) -> Result<(), String> {
    progress("Downloading NDI's official installer…");
    let (folder, installer) = download_installer()
        .map_err(|e| format!("Couldn't download the NDI engine installer: {e}. {MANUAL_HELP}"))?;
    let result = check_and_run(&installer, progress);
    let _ = std::fs::remove_dir_all(&folder);
    result
}

fn check_and_run(installer: &Path, progress: &dyn Fn(&str)) -> Result<(), String> {
    // Held until the installer has finished: while it is open, nobody can change or replace
    // the file, so what is checked is exactly what runs.
    let _locked = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(installer)
        .map_err(|e| format!("Couldn't open the downloaded installer: {e}. {MANUAL_HELP}"))?;
    progress("Checking the installer's digital signature…");
    let facts = installer_facts(installer)
        .map_err(|e| format!("Couldn't check the NDI installer: {e}. {MANUAL_HELP}"))?;
    super::installer_is_acceptable(&facts)
        .map_err(|e| format!("The downloaded NDI installer was rejected: {e}. {MANUAL_HELP}"))?;
    progress("Installing the NDI engine: allow the installer to make changes and accept NDI's licence…");
    run_installer(installer)
        .map_err(|e| format!("The NDI engine installer didn't run: {e}. {MANUAL_HELP}"))
}

/// Into a new folder of its own under the temp folder (refusing one that already exists), so
/// two overlays can't collide and nothing can be planted there beforehand.
fn download_installer() -> Result<(PathBuf, PathBuf), String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let folder = std::env::temp_dir().join(format!("uwh-overlay-ndi-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&folder).map_err(|e| e.to_string())?;
    let installer = folder.join("NDI 6 Runtime.exe");
    let written = fetch_into(&installer);
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&folder);
        return Err(e);
    }
    Ok((folder, installer))
}

fn fetch_into(installer: &Path) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("too many redirects")
                } else if super::redirect_allowed(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.error(format!("refused a redirect to {}", attempt.url()))
                }
            }))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(INSTALLER_URL)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| e.to_string())?;
        if let Some(length) = response.content_length() {
            super::within_size_limit(0, length)?;
        }
        let mut file = File::create_new(installer).map_err(|e| e.to_string())?;
        let mut so_far = 0;
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            so_far = super::within_size_limit(so_far, chunk.len() as u64)?;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
        }
        file.sync_all().map_err(|e| e.to_string())
    })
}

/// Asks Windows about the file's signature and product name. The path reaches PowerShell only
/// through an environment variable, so no file name can change the script.
fn installer_facts(installer: &Path) -> Result<super::InstallerFacts, String> {
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$path = $env:UWH_NDI_INSTALLER
$s = Get-AuthenticodeSignature -LiteralPath $path
$c = $s.SignerCertificate
$facts = [ordered]@{
  status = $s.Status.ToString()
  signer_name = if ($c) { $c.GetNameInfo('SimpleName', $false) } else { '' }
  signer_subject = if ($c) { @($c.SubjectName.Format($true) -split "`r?`n" | Where-Object { $_ }) } else { @() }
  issuer_name = if ($c) { $c.GetNameInfo('SimpleName', $true) } else { '' }
  product_name = [string](Get-Item -LiteralPath $path).VersionInfo.ProductName
}
ConvertTo-Json -InputObject $facts -Compress
"#;
    let output = powershell()
        .arg(SCRIPT)
        .env(PATH_VARIABLE, installer)
        .output()
        .map_err(|e| format!("couldn't run the signature check: {e}"))?;
    serde_json::from_slice(&output.stdout).map_err(|e| {
        format!(
            "unexpected answer from the signature check ({e}): {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })
}

/// Starts the installer with administrator rights (Windows shows its permission prompt) and
/// waits for it to finish.
fn run_installer(installer: &Path) -> Result<(), String> {
    const SCRIPT: &str = "Start-Process -FilePath $env:UWH_NDI_INSTALLER -Verb RunAs -Wait";
    let status = powershell()
        .arg(SCRIPT)
        .env(PATH_VARIABLE, installer)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("permission was refused or the installer couldn't start".to_string())
    }
}
```

  Also: `system_environment_value` runs `system32("reg.exe")` instead of `"reg"`. Delete `ps_quote`, `signature_is_acceptable` and the old `download_installer`/`check_signature`/`run_installer`. Update `MANUAL_HELP` to the exact string in Global Constraints. Leave `prepare_engine` calling `install_engine` for now (Task 2 restructures it). It then looks for the engine as before, so the program still works between tasks.

- [ ] **Step 7: Check that the Windows code compiles.** Run `cargo check -p overlay --features bridge --target x86_64-pc-windows-gnu --tests 2>&1 | grep -A5 ndi_runtime`. Expected: no output (no errors or warnings in `ndi_runtime.rs`). If `File::create_new` or `read_timeout` is reported missing, stop and report it: both exist in the locked versions (Rust ≥1.77; reqwest 0.12.23).

- [ ] **Step 8: Run `just check`.** Expected: exit 0.

---

### Task 2: NDI installs only when the operator clicks Install NDI; a broken engine never crashes the overlay (findings 3, 7, 9)

**Files:**
- Modify: `overlay/src/ndi_runtime.rs`
- Modify: `overlay/src/main.rs` (the NDI start-up block, the preview note, the mouse)
- Modify: `overlay/build.rs` (comment only: the DLL is preloaded by full path in `ndi_runtime.rs`)

**Interfaces:**
- Consumes: Task 1's `windows::install_engine(progress) -> Result<(), String>` and `MANUAL_HELP`.
- Produces:
  - `pub enum EngineStatus { Looking, Missing, Installing(String), Ready(Option<PathBuf>), Unavailable(String) }`
  - `EngineWatch::start() -> EngineWatch` (only looks; never installs)
  - `EngineWatch::install(&self)` (does nothing unless install is allowed)
  - `EngineWatch::begin_install(&self) -> bool`
  - `EngineWatch::ready(&self) -> Option<Option<PathBuf>>`
  - `pub fn install_allowed(status: &EngineStatus) -> bool`
  - `pub fn preview_note(status: &EngineStatus) -> String`
  - `with_engine_dir` is removed.

- [ ] **Step 1: Write the failing tests** (in `ndi_runtime.rs`'s `mod tests`):

```rust
#[test]
fn the_install_button_is_offered_only_when_ndi_is_missing_or_failed() {
    assert!(install_allowed(&EngineStatus::Missing));
    assert!(install_allowed(&EngineStatus::Unavailable("permission was refused".into())));
    assert!(!install_allowed(&EngineStatus::Looking));
    assert!(!install_allowed(&EngineStatus::Installing("Downloading…".into())));
    assert!(!install_allowed(&EngineStatus::Ready(None)));
}

#[test]
fn a_second_click_while_installing_starts_nothing() {
    let watch = EngineWatch::with_status(EngineStatus::Missing);
    assert!(watch.begin_install());
    assert!(matches!(watch.status(), EngineStatus::Installing(_)));
    assert!(!watch.begin_install());
}

#[test]
fn nothing_is_installed_until_the_operator_asks() {
    // Not missing yet: begin_install refuses, so nothing can start by itself.
    let watch = EngineWatch::with_status(EngineStatus::Looking);
    assert!(!watch.begin_install());
}

#[test]
fn the_preview_explains_each_state() {
    assert_eq!(
        preview_note(&EngineStatus::Missing),
        "NDI off: NDI isn't installed on this PC. Click Install NDI (Windows will ask for permission)."
    );
    assert_eq!(preview_note(&EngineStatus::Looking), "NDI: Looking for the NDI engine…");
    assert_eq!(preview_note(&EngineStatus::Installing("Downloading…".into())), "NDI: Downloading…");
    assert_eq!(preview_note(&EngineStatus::Unavailable("no".into())), "NDI off: no");
}
```

- [ ] **Step 2: Run them to see them fail.** Run `cargo test -p overlay ndi_runtime`. Expected: compile errors (`install_allowed`, `EngineWatch::with_status`, `begin_install`, `preview_note`, `EngineStatus::Missing` not found).

- [ ] **Step 3: Implement the state and the watch.** Replace `EngineStatus`, `EngineWatch`, `with_engine_dir` and the two `prepare_engine` functions with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    /// Looking for the engine at start-up.
    Looking,
    /// Not installed: the preview offers the Install NDI button.
    Missing,
    /// The operator clicked Install NDI; downloading, checking or installing.
    Installing(String),
    /// Loaded and ready. On Windows, `Some(folder)` is where it lives.
    Ready(Option<PathBuf>),
    /// Couldn't be installed or loaded; the preview says why and offers the button again.
    Unavailable(String),
}

/// Whether the Install NDI button is shown (and a click acted on).
pub fn install_allowed(status: &EngineStatus) -> bool {
    matches!(status, EngineStatus::Missing | EngineStatus::Unavailable(_))
}

/// The line shown on the local preview while NDI output isn't running.
pub fn preview_note(status: &EngineStatus) -> String {
    match status {
        EngineStatus::Looking => "NDI: Looking for the NDI engine…".to_string(),
        EngineStatus::Missing => "NDI off: NDI isn't installed on this PC. Click Install NDI (Windows will ask for permission).".to_string(),
        EngineStatus::Installing(message) => format!("NDI: {message}"),
        EngineStatus::Unavailable(message) => format!("NDI off: {message}"),
        EngineStatus::Ready(_) => "NDI off: couldn't start NDI output (see the log)".to_string(),
    }
}

#[derive(Clone)]
pub struct EngineWatch {
    status: Arc<Mutex<EngineStatus>>,
}

impl EngineWatch {
    /// Looks for the engine in the background. Never installs anything: that waits for the
    /// operator's click (see [`EngineWatch::install`]).
    pub fn start() -> Self {
        let watch = Self::with_status(EngineStatus::Looking);
        let background = watch.clone();
        std::thread::spawn(move || background.set(look_for_engine()));
        watch
    }

    fn with_status(status: EngineStatus) -> Self {
        Self { status: Arc::new(Mutex::new(status)) }
    }

    fn set(&self, status: EngineStatus) {
        if let EngineStatus::Unavailable(message) = &status {
            log::warn!("{message}");
        }
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }

    pub fn status(&self) -> EngineStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// `Some(folder)` once the engine is loaded.
    pub fn ready(&self) -> Option<Option<PathBuf>> {
        match self.status() {
            EngineStatus::Ready(dir) => Some(dir),
            _ => None,
        }
    }

    /// Moves to Installing if an install is allowed now; false if not (e.g. one is running).
    fn begin_install(&self) -> bool {
        let mut status = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if !install_allowed(&status) {
            return false;
        }
        *status = EngineStatus::Installing("Starting the NDI install…".to_string());
        true
    }

    /// The operator clicked Install NDI: installs in the background, then loads the engine.
    pub fn install(&self) {
        if !self.begin_install() {
            return;
        }
        let background = self.clone();
        std::thread::spawn(move || {
            let progress = |message: &str| {
                log::info!("{message}");
                background.set(EngineStatus::Installing(message.to_string()));
            };
            let status = match install_engine(&progress) {
                Ok(()) => match look_for_engine() {
                    EngineStatus::Missing => EngineStatus::Unavailable(format!(
                        "The NDI engine still isn't installed (was the installer cancelled?). {MANUAL_HELP}"
                    )),
                    other => other,
                },
                Err(message) => EngineStatus::Unavailable(message),
            };
            background.set(status);
        });
    }
}

#[cfg(not(windows))]
fn look_for_engine() -> EngineStatus {
    // Elsewhere the NDI library is found by the system's normal library search, as before.
    EngineStatus::Ready(None)
}

#[cfg(not(windows))]
fn install_engine(_progress: &dyn Fn(&str)) -> Result<(), String> {
    Err("Installing NDI is only automated on Windows".to_string())
}

#[cfg(windows)]
fn look_for_engine() -> EngineStatus {
    match windows::find_engine() {
        None => EngineStatus::Missing,
        Some(dir) => match windows::preload_engine(&dir) {
            Ok(()) => {
                log::info!("NDI engine loaded from {}", dir.display());
                EngineStatus::Ready(Some(dir))
            }
            Err(e) => EngineStatus::Unavailable(format!("{e}. {MANUAL_HELP}")),
        },
    }
}

#[cfg(windows)]
use windows::install_engine;
```

  In `mod windows`, make `find_engine` `pub(super)` and add:

```rust
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(name: *const u16, file: *mut std::ffi::c_void, flags: u32) -> *mut std::ffi::c_void;
}
/// Lets the engine's own dependencies be found in its folder.
const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x0000_0008;

/// Loads the engine by its full path before any NDI call. If Windows can't load it, the overlay
/// carries on without NDI and says why; otherwise the delayed load of
/// `Processing.NDI.Lib.x64.dll` (see `build.rs`) finds this already-loaded copy by name, so it
/// can no longer fail and crash the overlay. The module is never unloaded.
pub(super) fn preload_engine(dir: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let path = dir.join(ENGINE_DLL);
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `wide` is a NUL-terminated UTF-16 path that lives until the call returns; no file
    // handle is passed (null), and the flags are a documented LoadLibraryExW value.
    let module = unsafe {
        LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), LOAD_WITH_ALTERED_SEARCH_PATH)
    };
    if module.is_null() {
        Err(format!(
            "Windows couldn't load the NDI engine {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}
```

  Remove `prepare_engine` from `mod windows` (its steps now live in `look_for_engine` and `install_engine`). Update the module doc at the top of `ndi_runtime.rs` to describe: look only at start-up; install on the operator's click; preload by full path.

- [ ] **Step 4: Run the tests.** Run `cargo test -p overlay ndi_runtime`. Expected: PASS.

- [ ] **Step 5: Wire the preview and the button in `overlay/src/main.rs`.**
  - The NDI start block now starts output once the engine is loaded, with no directory juggling. It replaces the `with_engine_dir` call:

```rust
#[cfg(feature = "ndi")]
if !ndi_started && ndi_engine.ready().is_some() {
    ndi_started = true;
    ndi_output = match ndi_output::NdiOutput::new("UWH Overlay") {
        Ok(output) => {
            info!("NDI output started");
            Some(output)
        }
        Err(e) => {
            warn!("Failed to start NDI output, continuing without it: {e}");
            None
        }
    };
}
```

  - Replace the preview-note block with the following. Declare `let mut mouse_shown = false;` next to `ndi_started`. Keep `show_mouse(false)` at start-up.

```rust
// Only on the local preview, never in the NDI picture: why NDI isn't running, and the button
// that installs it. The mouse pointer shows only while the button does.
#[cfg(feature = "ndi")]
if ndi_output.is_none() {
    let status = ndi_engine.status();
    draw_text(&ndi_runtime::preview_note(&status), 10., 30., 24., YELLOW);
    let offer = ndi_runtime::install_allowed(&status);
    if offer != mouse_shown {
        macroquad::window::miniquad::window::show_mouse(offer);
        mouse_shown = offer;
    }
    if offer {
        let button = Rect::new(10., 44., 220., 40.);
        draw_rectangle(button.x, button.y, button.w, button.h, DARKGRAY);
        draw_rectangle_lines(button.x, button.y, button.w, button.h, 2., YELLOW);
        draw_text("Install NDI", button.x + 16., button.y + 28., 28., WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && button.contains(mouse_position().into()) {
            ndi_engine.install();
        }
    }
}
#[cfg(feature = "ndi")]
if ndi_output.is_some() && mouse_shown {
    macroquad::window::miniquad::window::show_mouse(false);
    mouse_shown = false;
}
```

  Add `Rect`, `draw_rectangle_lines`, `is_mouse_button_pressed`, `mouse_position`, `MouseButton` to the macroquad imports if `main.rs` doesn't already bring them in through `macroquad::prelude::*`.

- [ ] **Step 6: Update `overlay/build.rs`'s comment.** It should say that `ndi_runtime.rs` preloads the DLL by full path once found, so the delayed load never fails at the first NDI call.

- [ ] **Step 7: Compile-check Windows and run the gate.** Run `cargo check -p overlay --features bridge --target x86_64-pc-windows-gnu --tests 2>&1 | grep -A5 ndi_runtime`. Expected: no output. Then run `just check`. Expected: exit 0.

---

### Task 3: The bridge address moves from 8099 once, and the full-scan test comes back (findings 5, 6, 10)

**Files:**
- Modify: `overlay-bridge/src/config.rs`
- Modify: `overlay/src/main.rs` (`AppConfig`, the migration block)
- Modify: `overlay/src/network.rs:470` (the `AppConfig` destructuring pattern gets the new field)
- Modify: `overlay-bridge/src/discovery.rs`

**Interfaces:**
- Produces:
  - `Settings.port_moved_from_8099: Option<bool>` (bridge)
  - `AppConfig.bridge_port_moved: bool` with `#[serde(default)]` (overlay)
  - `fn moved_bridge_url(url: &str) -> Option<String>` (overlay `main.rs`)
  - `async fn scan_addresses(addresses: Vec<Ipv4Addr>, port: u16) -> Vec<Found>` (bridge `discovery.rs`)

- [ ] **Step 1: Write the failing tests.**
  - In `overlay-bridge/src/config.rs`, replace `a_saved_8099_moves_to_the_new_default_but_a_typed_one_is_kept` with:

```rust
#[test]
fn a_saved_8099_moves_once_and_a_later_choice_of_8099_is_kept() {
    // A settings file from before the move: 8099 was the old default, so it moves.
    let before = Settings { port: Some(8099), ..Settings::default() };
    let first = resolve_all(Overrides::default(), before, None);
    assert_eq!(first.port, DEFAULT_PORT);
    assert_eq!(DEFAULT_PORT, 8098);
    // What gets saved records that the move has happened.
    assert_eq!(first.to_settings().port_moved_from_8099, Some(true));

    // Later the operator deliberately types --port 8099; it is saved, and kept on the next start.
    let typed = Overrides { port: Some(8099), ..Overrides::default() };
    let chosen = resolve_all(typed, first.to_settings(), None).to_settings();
    assert_eq!(chosen.port, Some(8099));
    assert_eq!(resolve_all(Overrides::default(), chosen, None).port, 8099);

    // Any other saved port is left alone.
    let other = Settings { port: Some(9000), ..Settings::default() };
    assert_eq!(resolve_all(Overrides::default(), other, None).port, 9000);
}
```

  - In `overlay/src/main.rs`, add a `#[cfg(test)] mod tests` (or extend the existing one) with:

```rust
#[test]
fn any_bridge_address_on_the_old_port_moves_to_8098() {
    assert_eq!(moved_bridge_url("http://127.0.0.1:8099").as_deref(), Some("http://127.0.0.1:8098"));
    assert_eq!(moved_bridge_url("http://127.0.0.1:8099/").as_deref(), Some("http://127.0.0.1:8098/"));
    assert_eq!(moved_bridge_url("http://localhost:8099").as_deref(), Some("http://localhost:8098"));
    assert_eq!(moved_bridge_url("http://192.168.1.20:8099").as_deref(), Some("http://192.168.1.20:8098"));
    assert_eq!(moved_bridge_url("http://127.0.0.1:8098"), None);
    assert_eq!(moved_bridge_url("http://127.0.0.1:9000"), None);
    assert_eq!(moved_bridge_url("not a url"), None);
}

#[test]
fn an_old_settings_file_loads_and_is_marked_for_the_move() {
    // Written before this change: no `bridge_port_moved` key.
    let old: AppConfig = toml::from_str(
        "refbox_ip = \"127.0.0.1\"\nrefbox_port = 8000\nuwhportal_url = \"https://api.uwhportal.com\"\nbridge_url = \"http://127.0.0.1:8099\"\n",
    )
    .expect("an old settings file still loads");
    assert!(!old.bridge_port_moved);
    assert!(AppConfig::default().bridge_port_moved);
}
```

  If `toml` isn't a direct dependency of `overlay`, use `serde_json::from_str` with the equivalent JSON object instead. The point is that a missing key loads as `false`. Don't add a dependency.
  - In `overlay-bridge/src/discovery.rs`, restore the original full-scan test, calling the new helper:

```rust
#[tokio::test]
async fn a_full_subnet_scan_finds_a_real_refbox_and_finishes_in_a_few_seconds() {
    // A real refbox on 127.0.0.1, and 253 loopback addresses with nothing on them. Finding
    // the planted refbox is what proves the scan actually probed rather than returning an
    // empty list quickly -- a scan that did nothing at all would also be fast.
    let (address, refbox) = fake_refbox(second_half_snapshot()).await;

    let started = Instant::now();
    let all_254 = scan_targets(Ipv4Addr::new(127, 0, 0, 1)).collect();
    let found = scan_addresses(all_254, address.port).await;
    let elapsed = started.elapsed();

    assert_eq!(found.len(), 1, "exactly the planted refbox should have been found, got {found:?}");
    assert_eq!(found[0].address, address);
    assert!(
        elapsed < Duration::from_secs(5),
        "a full 254-address scan should finish in a few seconds, took {elapsed:?}"
    );

    refbox.abort();
}
```

  In `a_loopback_scan_finds_this_computers_refbox_exactly_once`, change the stale message to `"a scan of this computer should finish in a few seconds, took {elapsed:?}"`.

- [ ] **Step 2: Run them to see them fail.** Run `cargo test -p overlay-bridge config::tests discovery::tests` and `cargo test -p overlay moved_bridge_url an_old_settings_file`. Expected: compile errors (`port_moved_from_8099`, `scan_addresses`, `moved_bridge_url`, `bridge_port_moved` not found).

- [ ] **Step 3: Implement.**
  - Bridge `Settings`: add, after `roster_csv_path`:

```rust
    /// Set once the old default port 8099 has been moved to 8098. From then on a saved 8099
    /// is the operator's own choice and is kept (see [`OLD_DEFAULT_PORT`]).
    pub port_moved_from_8099: Option<bool>,
```

  - `resolve_all`'s port, replacing the filter:

```rust
        port: resolve(
            overrides.port,
            stored
                .port
                .filter(|port| *port != OLD_DEFAULT_PORT || stored.port_moved_from_8099 == Some(true)),
            defaults.port,
        ),
```

  - `to_settings` writes `port_moved_from_8099: Some(true)`. Update `OLD_DEFAULT_PORT`'s doc: a saved 8099 written before the move goes to 8098 once; after that, 8099 is respected.
  - Overlay `AppConfig`: add `#[serde(default)] bridge_port_moved: bool` (doc: "Set once an old `:8099` bridge address has been moved to 8098; a later 8099 is kept"). `Default` sets it to `true`, because a new settings file never held the old port. Add `bridge_port_moved: _` to the destructuring at `overlay/src/network.rs:470`.
  - Overlay `main.rs`, replacing `OLD_DEFAULT_BRIDGE_URL` and its block:

```rust
/// The old default port of the overlay-bridge; see `overlay-bridge`'s `config.rs`.
const OLD_BRIDGE_PORT: u16 = 8099;
const NEW_BRIDGE_PORT: u16 = 8098;

/// `url` with its port moved from 8099 to 8098, or `None` if it isn't on 8099. The rest of the
/// address is kept as written.
fn moved_bridge_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.port() != Some(OLD_BRIDGE_PORT) {
        return None;
    }
    let mut moved = parsed.clone();
    moved.set_port(Some(NEW_BRIDGE_PORT)).ok()?;
    let mut text = moved.to_string();
    if !url.ends_with('/') && moved.path() == "/" {
        text.pop();
    }
    Some(text)
}
```

  and in `main()`:

```rust
    // The bridge's default port moved from 8099 (vMix's hard-coded TCP API port) to 8098. A
    // settings file from before that is moved once, whatever host it names; after that, a
    // bridge address on 8099 is the operator's own choice and is kept.
    if !config.bridge_port_moved {
        if let Some(moved) = moved_bridge_url(&config.bridge_url) {
            info!("Moved the overlay-bridge address from {} to {moved}", config.bridge_url);
            config.bridge_url = moved;
        }
        config.bridge_port_moved = true;
        if let Err(e) = confy::store(APP_NAME, None, &config) {
            warn!("Couldn't save the overlay-bridge address: {e}");
        }
    }
```

  - `discovery.rs`: split `scan` so the probing lives in `scan_addresses`:

```rust
pub async fn scan(subnet: Ipv4Addr, port: u16) -> Vec<Found> {
    scan_addresses(targets_for(subnet), port).await
}

/// Probes exactly `addresses` (see [`scan`]), and returns the refboxes in address order.
async fn scan_addresses(addresses: Vec<Ipv4Addr>, port: u16) -> Vec<Found> {
    let targets: Vec<RefboxAddress> = addresses
        .into_iter()
        .map(|ip| RefboxAddress::new(ip.to_string(), port))
        .collect();
    let mut found = probe_all(targets, |address| async move {
        probe(&address, PROBE_TIMEOUT).await.ok()
    })
    .await;
    // Address order, not the order they happened to answer in, so the list an operator is reading
    // does not reshuffle itself between one scan and the next.
    found.sort_by_key(|f| f.address.host.parse::<Ipv4Addr>().ok());
    found
}
```

- [ ] **Step 4: Run the tests.** Run the two commands from Step 2. Expected: PASS.

- [ ] **Step 5: Run `just check`.** Expected: exit 0.

---

### Task 4 (controller): docs, PR body and the pre-PR checks

Not for a helper.
- The NDI step in `docs/streaming-setup.md` lives on the Stream Manager branch (`feat/workspace/stream-manager`). Change it there to "click **Install NDI** in the overlay's window and allow Windows' prompt", in its own commit.
- Rewrite the overlay PR body (scratchpad `overlay-pr-body.md`) to match: install on click, the checks, the one-time port move.
- Run the `security-review` skill and the `code-review` skill over `origin/master...HEAD` on this branch. Fix what they find; that work goes to a helper if needed.
- Write the user's walkthrough steps (Windows PC without NDI: Install NDI → prompt → No → button again → Yes → NDI output; an `overlay.toml` with `localhost:8099`; the bridge with a saved 8099).
- Then the dangling-work sweep, and push only on the user's OK.

## Deviations

(None yet.)
