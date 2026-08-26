//! Persists the bridge's own settings -- the refbox address it last connected to, the bridge's
//! own HTTP port, and the two operator settings side-of-pool and court (design spec §5.2) --
//! across runs, so an operator does not have to retype the same command-line flags every time the
//! bridge starts.
//!
//! Backed by [`confy`], the same crate `refbox` itself already uses for exactly this
//! (`refbox/src/main.rs:551`, `confy::load`/`confy::store`): a TOML file in the operating
//! system's standard per-user config directory, resolved via [`directories`] (confy's own
//! dependency), keyed by [`APP_NAME`]. Neither crate is new to the workspace --
//! `refbox/Cargo.toml` already pins `confy = "1.0"` and `directories = "6"`, and this task adds
//! the same versions here, plus `serde`'s `derive` feature (already used across `uwh-common` and
//! `refbox`) for [`Settings`] itself.
//!
//! # Precedence
//!
//! An explicitly-passed command-line argument beats a stored setting, which beats the built-in
//! default -- never the other way around, so an operator who types a flag this run is never
//! silently overridden by whatever they saved last session. [`resolve`] is the one place that
//! rule is implemented; `main.rs` calls it once per setting, then immediately saves the resolved
//! values back via [`store`] so they become "what was last used" for the *next* run with no
//! flags at all (design spec §5.3: "the last address used is remembered").
//!
//! # Missing vs. corrupt
//!
//! [`confy::load_path`] already treats a *missing* settings file as "use the defaults": the first
//! time it is asked to load a path with nothing there, it creates one from `Settings::default()`
//! and returns that, no error involved. A *corrupt* file -- one that exists but fails to parse,
//! whether hand-edited or damaged -- is different: `confy` returns an `Err` rather than silently
//! substituting defaults for content it could not read. [`load`] handles that explicitly, because
//! a settings file the bridge cannot parse must never stop it from starting: an operator who
//! cannot be expected to hand-repair a TOML file needs the bridge to fall back to defaults
//! instead, exactly as if the file were simply absent.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Identifies the bridge's own settings file to `confy`/`directories`, distinct from `refbox`'s
/// own `"refbox"` (`refbox/src/main.rs:54`) -- the two are separate applications with separate
/// config directories.
pub const APP_NAME: &str = "overlay-bridge";

/// Built-in defaults, used only once neither an explicit CLI argument nor a stored setting
/// supplies a value. Match the values `main.rs`'s CLI flags used before this task added
/// persistence (`--refbox-host` `127.0.0.1`, `--refbox-port` `8000`, `--port` `8099`), so a
/// first-ever run with no flags and no settings file behaves exactly as every run before this
/// task did.
pub const DEFAULT_REFBOX_HOST: &str = "127.0.0.1";
pub const DEFAULT_REFBOX_PORT: u16 = 8000;
pub const DEFAULT_PORT: u16 = 8099;

/// The bridge's persisted settings. Every field is `Option`, not a bare value with some
/// in-band "unset" sentinel: `Settings::default()` (every field `None`) is what a missing or
/// corrupt settings file resolves to (see the module doc), and `None` is what [`resolve`] treats
/// as "nothing stored yet, fall through to the built-in default" -- there is no other way to
/// represent "not set" for, say, `court`, if it were a bare `String`, without also using empty
/// string as a real value.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Hostname or IP address of the refbox last connected to.
    pub refbox_host: Option<String>,
    /// TCP port of the refbox last connected to.
    pub refbox_port: Option<u16>,
    /// The bridge's own HTTP server port.
    pub port: Option<u16>,
    /// Whether the white team is drawn on the physical right of the pool -- the "side of pool"
    /// operator setting the refbox feed cannot supply (design spec §5.2).
    pub white_on_right: Option<bool>,
    /// The court label -- the other operator setting the feed cannot supply (design spec §5.2).
    /// Free text (e.g. `"2"`, `"Pool A"`) rather than a number: venues name courts differently,
    /// and nothing in this crate parses it as anything but a label to display.
    pub court: Option<String>,
}

/// Resolves one setting under the bridge's standing precedence rule (see the module doc): an
/// explicitly-passed CLI argument beats a stored setting, which beats the built-in default.
pub fn resolve<T>(cli: Option<T>, stored: Option<T>, default: T) -> T {
    cli.or(stored).unwrap_or(default)
}

/// Where the bridge's settings file lives -- the OS-standard per-user config directory for
/// [`APP_NAME`], resolved by `confy`/`directories`.
fn settings_path() -> Result<PathBuf, confy::ConfyError> {
    confy::get_configuration_file_path(APP_NAME, None)
}

/// Loads the bridge's persisted settings, falling back to [`Settings::default`] if there is
/// nothing usable to load -- see the module doc's "Missing vs. corrupt" section. Never fails: a
/// settings problem must never stop the bridge from starting.
pub fn load() -> Settings {
    match settings_path() {
        Ok(path) => load_from(&path),
        Err(e) => {
            eprintln!(
                "could not determine where the bridge's settings file should live, starting \
                 with defaults: {e}"
            );
            Settings::default()
        }
    }
}

/// Saves `settings` so the next run of the bridge remembers them. Never fails loudly: a save
/// failure (a read-only filesystem, a permissions problem) is logged and otherwise ignored --
/// nothing about serving the bridge's live data depends on this succeeding.
pub fn store(settings: &Settings) {
    match settings_path() {
        Ok(path) => {
            if let Err(e) = confy::store_path(path, settings) {
                eprintln!("could not save the bridge's settings: {e}");
            }
        }
        Err(e) => eprintln!("could not determine where to save the bridge's settings: {e}"),
    }
}

/// The shared implementation behind [`load`] and this module's own tests: loads from an
/// arbitrary path (via `confy::load_path`) rather than the OS-standard one, so a test can point
/// it at a throwaway file instead of touching a real user's config directory. A *missing* path is
/// handled entirely inside `confy::load_path` itself (it creates one from `Settings::default()`
/// and returns that); a *corrupt* one returns `Err` from `confy`, which is caught here and turned
/// into `Settings::default()` -- see the module doc's "Missing vs. corrupt" section. Using this
/// same function for both production and tests (rather than duplicating the fallback logic)
/// means the corrupt-file test below proves the actual behaviour [`load`] has, not a parallel
/// implementation that could drift from it.
fn load_from(path: &Path) -> Settings {
    confy::load_path(path).unwrap_or_else(|e| {
        eprintln!("could not read the bridge's saved settings, starting with defaults: {e}");
        Settings::default()
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    /// A settings file path in the OS temp directory, unique per call so parallel tests never
    /// collide, and removed on drop so a test doesn't leave a stray file behind even if an
    /// assertion panics partway through.
    struct TempSettingsFile(PathBuf);

    impl TempSettingsFile {
        /// Reserves a unique path; does not create the file.
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "overlay-bridge-test-settings-{}-{n}.toml",
                std::process::id()
            ));
            Self(path)
        }
    }

    impl Drop for TempSettingsFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn settings_round_trip_through_save_and_load() {
        let file = TempSettingsFile::new();
        let settings = Settings {
            refbox_host: Some("192.168.1.50".to_string()),
            refbox_port: Some(9000),
            port: Some(9001),
            white_on_right: Some(true),
            court: Some("Pool A".to_string()),
        };

        confy::store_path(&file.0, &settings)
            .expect("store_path should succeed against a writable temp file");
        let loaded = load_from(&file.0);

        assert_eq!(
            loaded, settings,
            "every field saved must come back exactly as it was saved"
        );
    }

    #[test]
    fn a_missing_settings_file_yields_defaults_not_an_error() {
        let file = TempSettingsFile::new();
        assert!(
            !file.0.exists(),
            "test setup should not have created the file -- this test is specifically about the \
             file not existing yet"
        );

        let loaded = load_from(&file.0);

        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn a_corrupt_settings_file_yields_defaults_not_an_error() {
        let file = TempSettingsFile::new();
        // Deliberately not valid TOML -- an unbalanced inline table, not merely an unexpected
        // key, so this cannot be misread as "parses fine, into a slightly different shape".
        std::fs::write(&file.0, b"this is not valid TOML {{{ at all")
            .expect("writing the corrupt fixture file should succeed");

        let loaded = load_from(&file.0);

        assert_eq!(
            loaded,
            Settings::default(),
            "a settings file that exists but fails to parse must fall back to defaults, the same \
             as a missing one -- never an error that could stop the bridge from starting"
        );
    }

    #[test]
    fn resolve_prefers_the_cli_value_over_a_stored_one() {
        assert_eq!(resolve(Some(1), Some(2), 3), 1);
    }

    #[test]
    fn resolve_falls_back_to_the_stored_value_when_the_cli_did_not_supply_one() {
        assert_eq!(resolve(None, Some(2), 3), 2);
    }

    #[test]
    fn resolve_falls_back_to_the_built_in_default_when_nothing_else_supplied_a_value() {
        assert_eq!(resolve::<i32>(None, None, 3), 3);
    }
}
