//! Makes sure NDI®'s runtime (the "engine", `Processing.NDI.Lib.x64.dll`) is available before NDI
//! output starts, so a fresh streaming PC needs no manual NDI setup. NDI® is a registered
//! trademark of Vizrt NDI AB (https://ndi.video).
//!
//! On Windows the overlay is linked so that NDI's DLL is only loaded when NDI is first used (see
//! `build.rs`), which lets the overlay start without it. At startup, a background thread:
//! 1. looks for the engine where NDI's own installer puts it (and next to `overlay.exe`);
//! 2. if it isn't there, downloads NDI's official runtime installer (`ndi.link/NDIRedistV6`, the
//!    link NDI's distribution guidelines point applications to), refuses it unless Windows
//!    confirms it is validly signed by Vizrt, and starts it. Windows asks for permission and the
//!    person accepts NDI's licence in the installer;
//! 3. looks for the engine again once the installer has finished.
//!
//! The render loop polls [`EngineWatch::ready`] and starts NDI output as soon as the engine is
//! there, so no restart is needed after installing. If anything fails, the overlay keeps running
//! without NDI and says why.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    /// Still looking for, downloading or installing the engine.
    Preparing(String),
    /// The engine can be loaded. On Windows, `Some(folder)` is where it lives.
    Ready(Option<PathBuf>),
    /// No engine and it couldn't be installed; NDI output stays off until the next start.
    Unavailable(String),
}

#[derive(Clone)]
pub struct EngineWatch {
    status: Arc<Mutex<EngineStatus>>,
}

impl EngineWatch {
    /// Starts looking for (and if needed installing) the engine in the background.
    pub fn start() -> Self {
        let watch = Self {
            status: Arc::new(Mutex::new(EngineStatus::Preparing(
                "Looking for the NDI engine…".to_string(),
            ))),
        };
        let background = watch.clone();
        std::thread::spawn(move || {
            let result = prepare_engine(&|message: &str| {
                log::info!("{message}");
                background.set(EngineStatus::Preparing(message.to_string()));
            });
            match result {
                Ok(dir) => background.set(EngineStatus::Ready(dir)),
                Err(message) => {
                    log::warn!("{message}");
                    background.set(EngineStatus::Unavailable(message));
                }
            }
        });
        watch
    }

    fn set(&self, status: EngineStatus) {
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }

    pub fn status(&self) -> EngineStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// `Some(folder)` once the engine can be loaded.
    pub fn ready(&self) -> Option<Option<PathBuf>> {
        match self.status() {
            EngineStatus::Ready(dir) => Some(dir),
            _ => None,
        }
    }
}

/// Runs `start_ndi` (which makes the first NDI call, loading the engine) with Windows able to
/// find the engine's DLL in `dir`.
///
/// The DLL is loaded through the standard Windows search order, which includes the process's
/// current directory, so the current directory is pointed at the engine's folder just for that
/// first call and restored afterwards. Nothing else in the overlay uses relative file paths
/// (settings and logs use full paths), and once loaded the DLL stays loaded.
pub fn with_engine_dir<T>(dir: Option<&Path>, start_ndi: impl FnOnce() -> T) -> T {
    let Some(dir) = dir else {
        return start_ndi();
    };
    let previous = std::env::current_dir().ok();
    if let Err(e) = std::env::set_current_dir(dir) {
        log::warn!(
            "Couldn't switch to the NDI engine folder {}: {e}",
            dir.display()
        );
    }
    let result = start_ndi();
    if let Some(previous) = previous {
        let _ = std::env::set_current_dir(previous);
    }
    result
}

#[cfg(not(windows))]
fn prepare_engine(_progress: &dyn Fn(&str)) -> Result<Option<PathBuf>, String> {
    // Elsewhere the NDI library is found by the system's normal library search, as before.
    Ok(None)
}

#[cfg(windows)]
fn prepare_engine(progress: &dyn Fn(&str)) -> Result<Option<PathBuf>, String> {
    windows::prepare_engine(progress).map(Some)
}

#[cfg(windows)]
mod windows {
    use std::{
        path::{Path, PathBuf},
        process::Command,
    };

    pub const ENGINE_DLL: &str = "Processing.NDI.Lib.x64.dll";
    /// NDI's official, versioned link to its runtime installer.
    const INSTALLER_URL: &str = "https://ndi.link/NDIRedistV6";
    const RUNTIME_ENV: &str = "NDI_RUNTIME_DIR_V6";
    const DEFAULT_RUNTIME_DIR: &str = r"C:\Program Files\NDI\NDI 6 Runtime\v6";
    /// The organisation in the installer's code-signing certificate.
    const EXPECTED_SIGNER: &str = "Vizrt";
    const MANUAL_HELP: &str = "Install the NDI engine from https://ndi.link/NDIRedistV6 (or NDI Tools from \
         https://ndi.video/tools/), then restart the overlay.";

    pub fn prepare_engine(progress: &dyn Fn(&str)) -> Result<PathBuf, String> {
        if let Some(dir) = find_engine() {
            progress(&format!("NDI engine found in {}", dir.display()));
            return Ok(dir);
        }

        progress("The NDI engine isn't installed. Downloading NDI's official installer…");
        let installer = download_installer().map_err(|e| {
            format!("Couldn't download the NDI engine installer: {e}. {MANUAL_HELP}")
        })?;

        progress("Checking the installer's digital signature…");
        if let Err(e) = check_signature(&installer) {
            let _ = std::fs::remove_file(&installer);
            return Err(format!(
                "The downloaded NDI installer was rejected: {e}. {MANUAL_HELP}"
            ));
        }

        progress(
            "Installing the NDI engine: allow the installer to make changes and accept NDI's \
             licence…",
        );
        let ran = run_installer(&installer);
        let _ = std::fs::remove_file(&installer);
        ran.map_err(|e| format!("The NDI engine installer didn't run: {e}. {MANUAL_HELP}"))?;

        find_engine().ok_or_else(|| {
            format!(
                "The NDI engine still isn't installed (was the installer cancelled?). {MANUAL_HELP}"
            )
        })
    }

    /// Where NDI's installer puts the engine, in order of preference. The installer records the
    /// folder in a system environment variable; this process's own copy of the environment is
    /// from before any install it just ran, so the stored system value is read as well.
    fn find_engine() -> Option<PathBuf> {
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

    /// Reads a machine-wide environment variable as stored now (not as it was when this process
    /// started).
    fn system_environment_value(name: &str) -> Option<String> {
        let output = Command::new("reg")
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

    fn download_installer() -> Result<PathBuf, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let bytes = runtime.block_on(async {
            let response = reqwest::get(INSTALLER_URL).await?.error_for_status()?;
            response.bytes().await
        });
        let bytes = bytes.map_err(|e| e.to_string())?;
        let path = std::env::temp_dir().join("NDI 6 Runtime installer.exe");
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Single-quotes a path for PowerShell.
    fn ps_quote(path: &Path) -> String {
        format!("'{}'", path.display().to_string().replace('\'', "''"))
    }

    /// Asks Windows whether the file carries a valid code signature from Vizrt (NDI's owner).
    fn check_signature(installer: &Path) -> Result<(), String> {
        let script = format!(
            "$s = Get-AuthenticodeSignature -LiteralPath {}; \
             Write-Output ($s.Status.ToString() + '|' + $s.SignerCertificate.Subject)",
            ps_quote(installer)
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("couldn't run the signature check: {e}"))?;
        let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
        signature_is_acceptable(&answer)
    }

    /// `answer` is `<status>|<certificate subject>`.
    pub(super) fn signature_is_acceptable(answer: &str) -> Result<(), String> {
        let (status, subject) = answer.split_once('|').unwrap_or((answer, ""));
        if status != "Valid" {
            return Err(format!("its signature status is \"{status}\""));
        }
        if !subject.contains(EXPECTED_SIGNER) {
            return Err(format!(
                "it is signed by \"{subject}\", not {EXPECTED_SIGNER}"
            ));
        }
        Ok(())
    }

    /// Starts the installer with administrator rights (Windows shows its permission prompt) and
    /// waits for it to finish.
    fn run_installer(installer: &Path) -> Result<(), String> {
        let script = format!(
            "Start-Process -FilePath {} -Verb RunAs -Wait",
            ps_quote(installer)
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("permission was refused or the installer couldn't start".to_string())
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::windows::{parse_reg_query, signature_is_acceptable};

    #[test]
    fn reads_the_value_from_reg_query_output() {
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

    #[test]
    fn only_a_valid_vizrt_signature_is_accepted() {
        assert!(signature_is_acceptable("Valid|CN=Vizrt AG, O=Vizrt AG, L=Zürich, C=CH").is_ok());
        assert!(signature_is_acceptable("NotSigned|").is_err());
        assert!(signature_is_acceptable("HashMismatch|CN=Vizrt AG, O=Vizrt AG").is_err());
        assert!(signature_is_acceptable("Valid|CN=Someone Else").is_err());
    }
}
