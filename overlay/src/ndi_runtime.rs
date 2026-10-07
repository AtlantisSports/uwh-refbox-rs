#![cfg_attr(not(feature = "ndi"), allow(dead_code))]
//! Finds NDI®'s runtime (the "engine", `Processing.NDI.Lib.x64.dll`) before NDI output starts, and
//! installs it when the operator asks, so a fresh streaming PC needs no manual NDI setup. NDI® is
//! a registered trademark of Vizrt NDI AB (https://ndi.video).
//!
//! On Windows the overlay is linked so that NDI's DLL is only loaded when NDI is first used (see
//! `build.rs`), which lets the overlay start without it.
//! 1. At start-up a background thread only looks for the engine, where NDI's own installer puts
//!    it (and next to `overlay.exe`). It never installs anything by itself.
//! 2. Once found, the engine is loaded by its full path before any NDI call. If Windows can't load
//!    it, the overlay carries on without NDI and says why, rather than crashing at the first NDI
//!    call.
//! 3. If it isn't there, the local preview shows an **Install NDI** button. Only when the operator
//!    clicks it does the overlay download NDI's official runtime installer
//!    (`ndi.link/NDIRedistV6`, the link NDI's distribution guidelines point applications to),
//!    refuse it unless Windows confirms it is validly signed by Vizrt, and start it. Windows asks
//!    for permission and the person accepts NDI's licence in the installer. The engine is then
//!    looked for and loaded again.
//!
//! The render loop polls [`EngineWatch::ready`] and starts NDI output as soon as the engine is
//! loaded, so no restart is needed after installing. If anything fails, the overlay keeps running
//! without NDI, says why, and offers the button again.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Shown with every failure: how to retry, or install NDI by hand.
pub const MANUAL_HELP: &str = "Click Install NDI to try again, or install the NDI engine from https://ndi.link/NDIRedistV6 (or NDI Tools from https://ndi.video/tools/) and restart the overlay.";

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
        Self {
            status: Arc::new(Mutex::new(status)),
        }
    }

    fn set(&self, status: EngineStatus) {
        if let EngineStatus::Unavailable(message) = &status {
            log::warn!("{message}");
        }
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }

    pub fn status(&self) -> EngineStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
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
    if facts.status == "NotSigned" {
        return Err("it isn't signed".to_string());
    }
    if facts.status != "Valid" {
        return Err(format!("its signature status is \"{}\"", facts.status));
    }
    let organisation = format!("O={EXPECTED_SIGNER}");
    if facts.signer_name != EXPECTED_SIGNER
        || !facts
            .signer_subject
            .iter()
            .any(|part| part.trim() == organisation)
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
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| ALLOWED_HOSTS.contains(&host))
}

/// Adds `more` bytes to the `so_far` already downloaded, refusing anything past
/// [`MAX_INSTALLER_BYTES`].
pub fn within_size_limit(so_far: u64, more: u64) -> Result<u64, String> {
    so_far
        .checked_add(more)
        .filter(|total| *total <= MAX_INSTALLER_BYTES)
        .ok_or_else(|| "the download is far larger than NDI's installer".to_string())
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

#[cfg(windows)]
mod windows {
    use std::{
        fs::{File, OpenOptions},
        io::Write,
        os::windows::fs::OpenOptionsExt,
        path::{Path, PathBuf},
        process::Command,
        time::Duration,
    };

    use super::MANUAL_HELP;

    pub const ENGINE_DLL: &str = "Processing.NDI.Lib.x64.dll";
    /// NDI's official, versioned link to its runtime installer.
    const INSTALLER_URL: &str = "https://ndi.link/NDIRedistV6";
    const RUNTIME_ENV: &str = "NDI_RUNTIME_DIR_V6";
    const DEFAULT_RUNTIME_DIR: &str = r"C:\Program Files\NDI\NDI 6 Runtime\v6";
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
    const READ_TIMEOUT: Duration = Duration::from_secs(30);
    const TOTAL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
    /// Windows `FILE_SHARE_READ`: others may read the file, nobody may write or delete it.
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    /// The environment variable PowerShell reads the installer's path from.
    const PATH_VARIABLE: &str = "UWH_NDI_INSTALLER";

    /// Where NDI's installer puts the engine, in order of preference. The installer records the
    /// folder in a system environment variable; this process's own copy of the environment is
    /// from before any install it just ran, so the stored system value is read as well.
    pub(super) fn find_engine() -> Option<PathBuf> {
        let next_to_exe = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        [
            std::env::var_os(RUNTIME_ENV).map(PathBuf::from),
            system_environment_value(RUNTIME_ENV).map(PathBuf::from),
            Some(PathBuf::from(DEFAULT_RUNTIME_DIR)),
            next_to_exe,
        ]
        .into_iter()
        .flatten()
        .find(|dir| dir.join(ENGINE_DLL).is_file())
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryExW(
            name: *const u16,
            file: *mut std::ffi::c_void,
            flags: u32,
        ) -> *mut std::ffi::c_void;
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
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            )
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

    /// `%SystemRoot%\System32\<rest>`, so a same-named program elsewhere on the PATH is never run.
    fn system32(rest: &str) -> PathBuf {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        PathBuf::from(root).join("System32").join(rest)
    }

    fn powershell() -> Command {
        let mut command = Command::new(system32(r"WindowsPowerShell\v1.0\powershell.exe"));
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
        ]);
        command
    }

    /// Reads a machine-wide environment variable as stored now (not as it was when this process
    /// started).
    fn system_environment_value(name: &str) -> Option<String> {
        let output = Command::new(system32("reg.exe"))
            .args([
                "query",
                r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
                "/v",
                name,
            ])
            .output()
            .ok()?;
        parse_reg_query(&String::from_utf8_lossy(&output.stdout), name)
    }

    /// Picks the value out of `reg query` output such as
    /// `    NDI_RUNTIME_DIR_V6    REG_SZ    C:\Program Files\NDI\NDI 6 Runtime\v6`.
    pub(super) fn parse_reg_query(output: &str, name: &str) -> Option<String> {
        output.lines().find_map(|line| {
            let rest = line.trim_start().strip_prefix(name)?;
            let rest = rest.trim_start();
            let (_kind, value) = rest.split_once(char::is_whitespace)?;
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        })
    }

    /// Downloads, checks and runs NDI's runtime installer. Windows asks for permission; the person
    /// accepts NDI's licence in the installer.
    pub fn install_engine(progress: &dyn Fn(&str)) -> Result<(), String> {
        progress("Downloading NDI's official installer…");
        let (folder, installer) = download_installer().map_err(|e| {
            format!("Couldn't download the NDI engine installer: {e}. {MANUAL_HELP}")
        })?;
        let result = check_and_run(&installer, progress);
        remove_folder(&folder);
        result
    }

    fn remove_folder(folder: &Path) {
        if let Err(e) = std::fs::remove_dir_all(folder) {
            log::warn!(
                "Couldn't remove the NDI installer's temporary folder {}: {e}",
                folder.display()
            );
        }
    }

    fn check_and_run(installer: &Path, progress: &dyn Fn(&str)) -> Result<(), String> {
        // Held until the installer has finished: while it is open, the file can't be written to,
        // deleted or renamed, so it can't be changed or replaced between the check and the run.
        let _locked = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(installer)
            .map_err(|e| format!("Couldn't open the downloaded installer: {e}. {MANUAL_HELP}"))?;
        progress("Checking the installer's digital signature…");
        let facts = installer_facts(installer)
            .map_err(|e| format!("Couldn't check the NDI installer: {e}. {MANUAL_HELP}"))?;
        super::installer_is_acceptable(&facts).map_err(|e| {
            format!("The downloaded NDI installer was rejected: {e}. {MANUAL_HELP}")
        })?;
        progress(
            "Installing the NDI engine: allow the installer to make changes and accept NDI's \
             licence…",
        );
        run_installer(installer)
            .map_err(|e| format!("The NDI engine installer didn't run: {e}. {MANUAL_HELP}"))
    }

    /// Into a new folder of its own under the temp folder (refusing one that already exists), so
    /// two overlays can't collide and no file is already waiting there when the download starts.
    fn download_installer() -> Result<(PathBuf, PathBuf), String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let folder =
            std::env::temp_dir().join(format!("uwh-overlay-ndi-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&folder).map_err(|e| e.to_string())?;
        let installer = folder.join("NDI 6 Runtime.exe");
        let written = fetch_into(&installer);
        if let Err(e) = written {
            remove_folder(&folder);
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
                        let refused = format!("refused a redirect to {}", attempt.url());
                        attempt.error(refused)
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
  signer_subject = @(if ($c) { $c.SubjectName.Format($true) -split "`r?`n" | Where-Object { $_ } })
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
    ///
    /// The path is handed to .NET as is (never read as a wildcard pattern), and the installer's
    /// own exit code comes back as PowerShell's. A refused permission prompt throws, so
    /// PowerShell then exits non-zero too.
    fn run_installer(installer: &Path) -> Result<(), String> {
        const SCRIPT: &str = "$ErrorActionPreference = 'Stop'; $i = New-Object System.Diagnostics.ProcessStartInfo; $i.FileName = $env:UWH_NDI_INSTALLER; $i.Verb = 'runas'; $i.UseShellExecute = $true; $p = [System.Diagnostics.Process]::Start($i); $p.WaitForExit(); exit $p.ExitCode";
        let output = powershell()
            .arg(SCRIPT)
            .env(PATH_VARIABLE, installer)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            log::warn!("The NDI installer launch reported: {}", stderr.trim());
        }
        Err(match output.status.code() {
            Some(code) => format!(
                "permission was refused, the installer couldn't start, or it ended with exit code {code}"
            ),
            None => "permission was refused or the installer couldn't start".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn reads_the_value_from_reg_query_output() {
        use super::windows::parse_reg_query;
        let output = "\r\nHKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment\r\n    NDI_RUNTIME_DIR_V6    REG_SZ    C:\\Program Files\\NDI\\NDI 6 Runtime\\v6\r\n\r\n";
        assert_eq!(
            parse_reg_query(output, "NDI_RUNTIME_DIR_V6").as_deref(),
            Some("C:\\Program Files\\NDI\\NDI 6 Runtime\\v6")
        );
        assert_eq!(parse_reg_query(output, "NDI_RUNTIME_DIR_V5"), None);
        assert_eq!(
            parse_reg_query("ERROR: not found", "NDI_RUNTIME_DIR_V6"),
            None
        );
    }

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

        let not_valid = InstallerFacts {
            status: "HashMismatch".into(),
            ..genuine()
        };
        assert!(installer_is_acceptable(&not_valid).is_err());

        // A file with no signature at all: the reason says so plainly.
        let unsigned = InstallerFacts {
            status: "NotSigned".into(),
            signer_name: String::new(),
            signer_subject: vec![],
            issuer_name: String::new(),
            ..genuine()
        };
        assert_eq!(
            installer_is_acceptable(&unsigned),
            Err("it isn't signed".to_string())
        );

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
        let other_product = InstallerFacts {
            product_name: "Vizrt Viz Engine".into(),
            ..genuine()
        };
        assert!(installer_is_acceptable(&other_product).is_err());

        // A certificate not issued by DigiCert.
        let other_issuer = InstallerFacts {
            issuer_name: "Some Test CA".into(),
            ..genuine()
        };
        assert!(installer_is_acceptable(&other_issuer).is_err());
    }

    #[test]
    fn the_download_only_follows_https_links_to_ndis_own_sites() {
        let ok = |u: &str| redirect_allowed(&reqwest::Url::parse(u).unwrap());
        assert!(ok("https://ndi.link/NDIRedistV6"));
        assert!(ok(
            "https://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20Runtime.exe"
        ));
        assert!(!ok(
            "http://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20Runtime.exe"
        ));
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

    #[test]
    fn the_install_button_is_offered_only_when_ndi_is_missing_or_failed() {
        assert!(install_allowed(&EngineStatus::Missing));
        assert!(install_allowed(&EngineStatus::Unavailable(
            "permission was refused".into()
        )));
        assert!(!install_allowed(&EngineStatus::Looking));
        assert!(!install_allowed(&EngineStatus::Installing(
            "Downloading…".into()
        )));
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
        assert_eq!(
            preview_note(&EngineStatus::Looking),
            "NDI: Looking for the NDI engine…"
        );
        assert_eq!(
            preview_note(&EngineStatus::Installing("Downloading…".into())),
            "NDI: Downloading…"
        );
        assert_eq!(
            preview_note(&EngineStatus::Unavailable("no".into())),
            "NDI off: no"
        );
    }
}
