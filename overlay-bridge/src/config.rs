//! Persists the bridge's own settings -- the refbox address it last connected to, the bridge's
//! own HTTP port, and the two operator settings side-of-pool and court (design spec §5.2) --
//! across runs, so an operator does not have to retype the same command-line flags every time the
//! bridge starts.
//!
//! Backed by [`confy`], the same crate `refbox` itself already uses for exactly this
//! (`refbox/src/main.rs:551`, `confy::load`/`confy::store`): a TOML file in the operating
//! system's standard per-user config directory, resolved internally by confy via its own
//! `directories` dependency (not a direct dependency of this crate -- nothing here calls
//! `directories` itself, only `confy`'s own path-resolution functions), keyed by [`APP_NAME`].
//! `confy` is not new to the workspace -- `refbox/Cargo.toml` already pins `confy = "1.0"`, and
//! this task adds the same version here, plus `serde`'s `derive` feature (already used across
//! `uwh-common` and `refbox`) for [`Settings`] itself.
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

use crate::feed::RefboxAddress;

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

/// Whatever the operator actually typed on the command line this run. Every field is `Option`
/// because "not passed" has to be distinguishable from "passed, with the same value as the
/// default" -- see the module doc's precedence section. `main.rs`'s `clap`-derived `Cli` converts
/// itself into this, and nothing else about `clap` reaches the rest of the crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    pub refbox_host: Option<String>,
    pub refbox_port: Option<u16>,
    pub port: Option<u16>,
    pub white_on_right: Option<bool>,
    pub court: Option<String>,
}

/// Every setting the running bridge needs, already resolved -- **one value per setting, with no
/// way to leave one out.**
///
/// This shape is deliberate, and it is the fix for a class of bug rather than an instance of one
/// (Task 8 review, Important 3). The settings used to reach `server::AppState` through optional
/// builder methods, so deleting one line in `main.rs` left the bridge running with a *default*
/// where a configured value belonged -- silently connecting to `127.0.0.1:8000` no matter what
/// `--refbox-host` or the saved settings said, with every test still green, because nothing about
/// a missing builder call is visible to a compiler or to a test of anything else. A plain struct
/// with no optional fields makes that impossible: a setting can be wrong, but it can no longer be
/// *absent*, and [`resolve_all`] is the single place any of them is decided.
///
/// [`Default`] is the bridge's built-in configuration (`127.0.0.1:8000`, HTTP on 8099, white on
/// the left, no court, nothing remembered anywhere) -- genuinely what a first-ever run with no
/// flags and no settings file uses, which is also what makes it the honest starting point for a
/// test that cares about only one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Which refbox to read at startup. Only the *starting* point: the operator can choose a
    /// different one from the status page at any time (see `feed::FeedTarget`).
    pub refbox: RefboxAddress,
    /// The bridge's own HTTP port, for vMix and the status page.
    pub port: u16,
    pub white_on_right: bool,
    pub court: String,
    /// Where to remember a refbox chosen at runtime, or `None` if the settings file's location
    /// could not be worked out (a choice then still applies for this run -- the status page says
    /// plainly that it will not be remembered).
    pub settings_path: Option<PathBuf>,
}

impl Default for Resolved {
    fn default() -> Self {
        Self {
            refbox: RefboxAddress::new(DEFAULT_REFBOX_HOST, DEFAULT_REFBOX_PORT),
            port: DEFAULT_PORT,
            white_on_right: false,
            court: String::new(),
            settings_path: None,
        }
    }
}

impl Resolved {
    /// The persisted form of these settings, for writing back what was actually used this run so
    /// it becomes "what was last used" next time (design spec §5.3).
    pub fn to_settings(&self) -> Settings {
        Settings {
            refbox_host: Some(self.refbox.host.clone()),
            refbox_port: Some(self.refbox.port),
            port: Some(self.port),
            white_on_right: Some(self.white_on_right),
            court: Some(self.court.clone()),
        }
    }
}

/// Applies the precedence rule to every setting at once: an explicitly-passed command-line
/// argument beats a stored one, which beats the built-in default.
///
/// The whole of it, in one place, so "where does the bridge get the refbox address from" has
/// exactly one answer that a test can call directly -- see [`Resolved`] for the bug that made
/// this worth extracting from `main.rs`.
pub fn resolve_all(
    overrides: Overrides,
    stored: Settings,
    settings_path: Option<PathBuf>,
) -> Resolved {
    let defaults = Resolved::default();
    Resolved {
        refbox: RefboxAddress::new(
            resolve(
                overrides.refbox_host,
                stored.refbox_host,
                defaults.refbox.host,
            ),
            resolve(
                overrides.refbox_port,
                stored.refbox_port,
                defaults.refbox.port,
            ),
        ),
        port: resolve(overrides.port, stored.port, defaults.port),
        white_on_right: resolve(
            overrides.white_on_right,
            stored.white_on_right,
            defaults.white_on_right,
        ),
        court: resolve(overrides.court, stored.court, defaults.court),
        settings_path,
    }
}

/// Where the bridge's settings file lives -- the OS-standard per-user config directory for
/// [`APP_NAME`], resolved by `confy` (via its own `directories` dependency).
///
/// Public since Task 8: `main.rs` resolves this once at startup and hands it to the HTTP server,
/// so that an address the operator chooses while the bridge is running is remembered in the same
/// file this one names -- see [`remember_refbox_address`].
pub fn settings_path() -> Result<PathBuf, confy::ConfyError> {
    confy::get_configuration_file_path(APP_NAME, None)
}

/// A human-readable description of where the settings file lives, for the operator status page
/// (Task 7 review, Important 3's floor: the page must tell an operator where to go to fix a
/// mistyped setting even though it cannot edit one itself yet -- see `status`'s module doc's "The
/// address is not a dead end" section). Falls back to an explanatory placeholder rather than an
/// error if the path can't be determined -- this is display text, not something the bridge's own
/// operation depends on succeeding.
pub fn settings_location() -> String {
    match settings_path() {
        Ok(path) => path.display().to_string(),
        Err(e) => format!("(could not be determined: {e})"),
    }
}

/// Everything `main` does about settings, in one call: load whatever was saved last time, decide
/// this run's value for every setting under the precedence rule (see the module doc), and write
/// the result straight back so it becomes "what was last used" for the next run with no flags at
/// all (design spec §5.3).
///
/// **One call rather than three, deliberately** (final review, Minor 9). These were previously
/// three separate steps written out in `main.rs`, which is the one file in this crate no test can
/// reach: leaving out the save, or resolving against something other than what was loaded, was a
/// one-line mistake that nothing would have caught. Everything here except finding the settings
/// file is [`resolve_and_store_at`], which a test drives against a throwaway path.
///
/// Never fails: a settings problem must never stop the bridge from starting. If the settings
/// file's location cannot be worked out at all, this run uses typed flags over built-in defaults
/// and nothing is remembered -- and the status page says so plainly rather than promising a save
/// that will not happen.
pub fn load_resolve_and_store(overrides: Overrides) -> Resolved {
    let path = match settings_path() {
        Ok(path) => Some(path),
        Err(e) => {
            eprintln!(
                "could not determine where the bridge's settings file should live, starting \
                 with defaults and remembering nothing: {e}"
            );
            None
        }
    };
    resolve_and_store_at(overrides, path)
}

/// The whole of [`load_resolve_and_store`] except working out where the settings file lives, so
/// a test can point it at a throwaway file instead of a real user's config directory. `None`
/// means there is nowhere to load from or save to: the run still gets a complete [`Resolved`].
fn resolve_and_store_at(overrides: Overrides, path: Option<PathBuf>) -> Resolved {
    let stored = path.as_deref().map(load_from).unwrap_or_default();
    let resolved = resolve_all(overrides, stored, path);
    if let Some(path) = &resolved.settings_path {
        store_at(path, &resolved.to_settings());
    }
    resolved
}

/// Saves `settings` to `path`. Never fails loudly: a save failure (a read-only filesystem, a
/// permissions problem) is logged and otherwise ignored -- nothing about serving the bridge's
/// live data depends on this succeeding.
fn store_at(path: &Path, settings: &Settings) {
    if let Err(e) = confy::store_path(path, settings) {
        eprintln!("could not save the bridge's settings: {e}");
    }
}

/// Remembers a refbox address the operator chose while the bridge was running (Task 8), in the
/// settings file at `path`, leaving every other setting exactly as it is.
///
/// Read-modify-write rather than writing a whole [`Settings`] built from what the running process
/// happens to know: the bridge's own HTTP port, side of pool and court all live in the same file,
/// and writing the file from a partial picture would silently clear whichever of them the caller
/// did not have to hand. The read is the file's current contents, so the only field this can ever
/// change is the address.
///
/// Never fails loudly, for the same reason as [`store`]: an address the operator chose is already
/// in use for this run whether or not it could be written down for the next one, and a settings
/// problem must not interrupt a broadcast. A failure is reported to stderr and to the caller (as
/// `false`) so the status page can say plainly that the choice will not survive a restart, rather
/// than silently promising it will.
pub fn remember_refbox_address(path: &Path, host: &str, port: u16) -> bool {
    let settings = Settings {
        refbox_host: Some(host.to_string()),
        refbox_port: Some(port),
        ..load_from(path)
    };
    match confy::store_path(path, &settings) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("could not remember the chosen refbox address: {e}");
            false
        }
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

    // ------------------------------------------------------------------ remember_refbox_address

    #[test]
    fn a_chosen_refbox_address_is_remembered_for_the_next_run() {
        // The point of the whole thing: an operator who picks a refbox on the status page today
        // finds the bridge already pointed at it tomorrow, without typing anything. `load_from`
        // plus `resolve` here is exactly what `main.rs` does on a run with no flags at all.
        let file = TempSettingsFile::new();
        confy::store_path(
            &file.0,
            &Settings {
                refbox_host: Some("127.0.0.1".to_string()),
                refbox_port: Some(8000),
                ..Settings::default()
            },
        )
        .expect("writing the starting settings should succeed");

        assert!(remember_refbox_address(&file.0, "192.168.1.50", 8123));

        let next_run = load_from(&file.0);
        assert_eq!(
            resolve(None, next_run.refbox_host, DEFAULT_REFBOX_HOST.to_string()),
            "192.168.1.50"
        );
        assert_eq!(
            resolve(None, next_run.refbox_port, DEFAULT_REFBOX_PORT),
            8123
        );
    }

    #[test]
    fn remembering_an_address_leaves_every_other_setting_alone() {
        // The read-modify-write is the whole reason this function exists rather than a `store` of
        // a freshly-built `Settings`: the HTTP port, side of pool and court are set elsewhere and
        // must survive a change of refbox. Without the read, this test finds them cleared.
        let file = TempSettingsFile::new();
        let before = Settings {
            refbox_host: Some("127.0.0.1".to_string()),
            refbox_port: Some(8000),
            port: Some(9001),
            white_on_right: Some(true),
            court: Some("Pool A".to_string()),
        };
        confy::store_path(&file.0, &before).expect("writing the starting settings should succeed");

        assert!(remember_refbox_address(&file.0, "192.168.1.50", 8000));

        let after = load_from(&file.0);
        assert_eq!(after.port, before.port);
        assert_eq!(after.white_on_right, before.white_on_right);
        assert_eq!(after.court, before.court);
        assert_eq!(after.refbox_host.as_deref(), Some("192.168.1.50"));
    }

    #[test]
    fn remembering_an_address_reports_failure_rather_than_pretending_it_saved() {
        // A path that cannot be written to (a directory where a file should be). The bridge must
        // carry on -- the address is in use either way -- but it must not tell the operator the
        // choice will survive a restart when it will not.
        let dir = std::env::temp_dir().join(format!(
            "overlay-bridge-test-unwritable-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("creating the blocking directory should succeed");

        let saved = remember_refbox_address(&dir, "192.168.1.50", 8000);

        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            !saved,
            "storing into a path that is a directory cannot succeed, and must be reported"
        );
    }

    // ------------------------------------------------------------------------------ resolve_all

    #[test]
    fn every_setting_follows_the_same_precedence_rule_at_once() {
        // One test for the whole resolution rather than one per field: `resolve_all` is now the
        // single place any setting is decided (see `Resolved`), so what matters is that a typed
        // flag, a stored value and a built-in default each win where they should -- across all of
        // them together, in one call, the way `main` makes it.
        let overrides = Overrides {
            refbox_host: Some("192.168.1.50".to_string()),
            court: Some("Pool B".to_string()),
            ..Overrides::default()
        };
        let stored = Settings {
            refbox_host: Some("10.0.0.9".to_string()),
            refbox_port: Some(8123),
            port: Some(9001),
            white_on_right: Some(true),
            court: Some("Pool A".to_string()),
        };

        let resolved = resolve_all(overrides, stored, Some(PathBuf::from("/tmp/settings.toml")));

        // Typed this run: wins.
        assert_eq!(resolved.refbox.host, "192.168.1.50");
        assert_eq!(resolved.court, "Pool B");
        // Not typed, but stored: the stored value wins over the built-in default.
        assert_eq!(resolved.refbox.port, 8123);
        assert_eq!(resolved.port, 9001);
        assert!(resolved.white_on_right);
        assert_eq!(
            resolved.settings_path,
            Some(PathBuf::from("/tmp/settings.toml"))
        );
    }

    #[test]
    fn with_nothing_typed_and_nothing_stored_every_setting_is_the_built_in_default() {
        let resolved = resolve_all(Overrides::default(), Settings::default(), None);

        assert_eq!(resolved, Resolved::default());
        assert_eq!(resolved.refbox.host, DEFAULT_REFBOX_HOST);
        assert_eq!(resolved.refbox.port, DEFAULT_REFBOX_PORT);
        assert_eq!(resolved.port, DEFAULT_PORT);
    }

    #[test]
    fn what_was_resolved_is_what_gets_written_back_for_next_time() {
        // `main` saves the resolved settings on every run so they become "what was last used"
        // (design spec §5.3). Every field must make that round trip, or a setting the operator
        // passed once would be forgotten while looking as though it had been saved.
        let resolved = Resolved {
            refbox: RefboxAddress::new("192.168.1.50", 8123),
            port: 9001,
            white_on_right: true,
            court: "Pool B".to_string(),
            settings_path: None,
        };

        let round_tripped = resolve_all(Overrides::default(), resolved.to_settings(), None);

        assert_eq!(
            round_tripped,
            Resolved {
                settings_path: None,
                ..resolved
            }
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

    #[test]
    fn settings_location_names_the_app_so_it_is_not_a_generic_or_empty_path() {
        // Not asserting an exact path -- that's platform-dependent (XDG on Linux, AppData on
        // Windows, etc., all via confy's own `directories` dependency) -- but the app name must
        // appear somewhere in it, and it must not be empty, or the status page's "where to go to
        // fix a mistyped setting" hint (see `status`'s module doc) would be useless.
        let location = settings_location();
        assert!(!location.is_empty());
        assert!(
            location.contains(APP_NAME),
            "settings_location() should mention {APP_NAME:?}, got {location:?}"
        );
    }
}
