#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::Parser;
use i18n_embed::{
    DesktopLanguageRequester,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use iced::{
    Settings, Size,
    window::{self, icon},
};
use iced_core::Font;
use log::*;
#[cfg(debug_assertions)]
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::{
    append::rolling_file::{
        RollingFileAppender,
        policy::compound::{
            CompoundPolicy, roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
        },
    },
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use once_cell::sync::Lazy;
use rust_embed::RustEmbed;
use std::{
    borrow::Cow,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
};
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};
use unic_langid::LanguageIdentifier;

mod app;
mod app_icon;
mod beep_test;
mod penalty_editor;
mod portal_manager;
mod sim_app;
mod sim_frame;
mod sound_controller;
mod tournament_manager;
pub mod updater;

mod config;
use app::languages::Language;
use config::{Config, Mode};

const APP_NAME: &str = "refbox";

#[derive(RustEmbed)]
#[folder = "translations/"]
struct Localizations;

static LANGUAGE_OVERRIDE: Lazy<Arc<std::sync::Mutex<Option<LanguageIdentifier>>>> =
    Lazy::new(|| Arc::new(std::sync::Mutex::new(None)));

static LANGUAGE_LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    let requested_languages = if let Some(lang) = LANGUAGE_OVERRIDE.lock().unwrap().take() {
        info!("Using language override: {lang:?}");
        vec![lang]
    } else {
        DesktopLanguageRequester::requested_languages()
    };

    request_language(&loader, &requested_languages);

    loader
});

fn request_language(loader: &FluentLanguageLoader, requested_languages: &[LanguageIdentifier]) {
    match i18n_embed::select(loader, &Localizations, requested_languages) {
        Ok(lang) => info!("Using language: {lang:?}"),
        Err(e) => warn!(
            "Unable to select languages: {e}\nRequested languages were: {requested_languages:?}"
        ),
    }

    loader.set_use_isolating(false); // Required until iced supports RTL text (https://github.com/iced-rs/iced/issues/250)
}

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, short)]
    /// Don't run the simulator GUI
    no_simulate: bool,

    #[clap(long, short, action(clap::ArgAction::Count))]
    /// Increase the log verbosity
    verbose: u8,

    #[clap(long, short, default_value = "4")]
    /// Size of a pixel in the panel the simulator
    scale: f32,

    #[clap(long)]
    /// Spacing between pixels in the panel the simulator
    spacing: Option<f32>,

    #[clap(long, short)]
    /// Make the app fullscreen
    fullscreen: bool,

    #[clap(long, default_value = "8001")]
    /// Port to listen on for TCP connections with a binary send type
    binary_port: u16,

    #[clap(long, default_value = "8000")]
    /// Port to listen on for TCP connections with a JSON send type
    json_port: u16,

    #[clap(long, default_missing_value = "/dev/ttyUSB0")]
    /// Serial Port to send snapshots to
    serial_port: Option<String>,

    #[clap(long, default_value = "115200")]
    /// Baud rate for the serial port
    baud_rate: u32,

    #[clap(long)]
    /// Don't require HTTPS to connect to uwhportal
    allow_http: bool,

    #[clap(long, short)]
    /// List all events from uwhportal, including past ones
    all_events: bool,

    #[clap(long)]
    /// Directory within which log files will be placed, default is platform dependent
    log_location: Option<PathBuf>,

    #[clap(long, default_value = "5000000")]
    /// Max size in bytes that a log file is allowed to reach before being rolled over
    log_max_file_size: u64,

    #[clap(long, default_value = "3")]
    /// Number of archived logs to keep
    num_old_logs: u32,

    #[clap(long)]
    /// Override the system language (ex: en-US, fr, es, etc.)
    language: Option<LanguageIdentifier>,

    #[clap(long, hide = true)]
    is_simulator: bool,

    #[clap(long, hide = true, value_name = "DIR")]
    /// Dev-time: render every front-display layout preview PNG into DIR, then exit.
    capture_previews: Option<PathBuf>,

    #[clap(long, hide = true)]
    simulate_sunlight_display: bool,

    #[clap(long, hide = true)]
    /// Probe that the binary starts on this machine, then exit 0. Used as the
    /// post-download smoke test before committing to a new binary.
    self_check: bool,
}

/// All arguments needed to launch a panel-simulator child process. Built once
/// in `main()` from the parsed CLI, then reused for every sim window we spawn
/// (the startup one, and any opened later via the Display Options button).
#[derive(Debug, Clone)]
pub struct SimSpawnConfig {
    pub binary_port: u16,
    pub json_port: u16,
    pub scale: f32,
    pub spacing: f32,
    pub sunlight_mode: bool,
    pub verbose: u8,
    pub log_location: PathBuf,
    pub log_max_file_size: u64,
    pub num_old_logs: u32,
}

/// Build the argv that `spawn_sim_child` passes to the spawned process.
/// Factored out as a pure function so its construction can be unit-tested
/// without spawning.
pub fn build_sim_argv(config: &SimSpawnConfig) -> Vec<String> {
    let mut args = vec![
        "--is-simulator".to_string(),
        "--binary-port".to_string(),
        config.binary_port.to_string(),
        "--json-port".to_string(),
        config.json_port.to_string(),
        "--scale".to_string(),
        config.scale.to_string(),
        "--spacing".to_string(),
        config.spacing.to_string(),
        "--log-location".to_string(),
        // Matches the original main.rs behaviour. A non-UTF-8 log path would
        // already have panicked at startup before we got here.
        config.log_location.to_str().unwrap().to_string(),
        "--log-max-file-size".to_string(),
        config.log_max_file_size.to_string(),
        "--num-old-logs".to_string(),
        config.num_old_logs.to_string(),
    ];
    for _ in 0..config.verbose {
        args.push("--verbose".to_string());
    }
    if config.sunlight_mode {
        args.push("--simulate-sunlight-display".to_string());
    }
    args
}

/// Build the argv used to relaunch the MAIN app on an in-app restart so the new
/// process inherits the same command-line settings the operator started with
/// (fullscreen, serial/LED-panel port, real-hardware mode, ports, logging).
///
/// Deliberately NOT replayed:
///  - `--language`: a restart is often triggered BY a language change. The new
///    language is persisted to the config file and must drive startup; replaying
///    the old `--language` would override it.
///  - `--is-simulator`: this relaunches the main app, never a simulator child.
///  - `--capture-previews`: a dev-only flag that renders preview PNGs and exits
///    immediately; replaying it would make the restarted process exit at once.
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
        self_check: _,       // a smoke-test probe, never replayed into a real restart
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

pub(crate) fn spawn_sim_child(config: &SimSpawnConfig) -> std::io::Result<std::process::Child> {
    let bin_name = std::env::current_exe()?.into_os_string();
    let argv = build_sim_argv(config);
    info!("Spawning sim child, bin_name: {bin_name:?}, args: {argv:?}");
    Command::new(bin_name)
        .args(&argv)
        .stdin(Stdio::null())
        .spawn()
}

/// Respawn the app: `exe` is the program file to launch, `argv` the replayed
/// start-up arguments. A failed spawn is logged, never silently swallowed.
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

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    // Capture the argv needed to relaunch this same configuration on an in-app
    // restart, before any field of `args` is moved below.
    let restart_argv = build_restart_argv(&args);

    if let Some(lang) = args.language {
        *LANGUAGE_OVERRIDE.lock().unwrap() = Some(lang);
    }

    let log_level = match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut log_base_path = args.log_location.unwrap_or_else(|| {
        let mut path = directories::BaseDirs::new()
            .expect("Could not find a directory to store logs")
            .data_local_dir()
            .to_path_buf();
        path.push("uwh-refbox-logs");
        path
    });
    let app_name = if args.is_simulator {
        log_base_path.push("simulator");
        "simulator"
    } else {
        "refbox"
    };
    let mut log_path = log_base_path.clone();
    let mut archived_log_path = log_base_path.clone();
    log_path.push(format!("{app_name}-log.txt"));
    archived_log_path.push(format!("{app_name}-log-{{}}.txt.gz"));

    #[cfg(debug_assertions)]
    if !args.is_simulator {
        println!("Log path: {}", log_path.display());
    }

    // Only log to the console in debug mode
    #[cfg(all(debug_assertions, not(target_os = "windows")))]
    let console_target = Target::Stderr;
    #[cfg(all(debug_assertions, target_os = "windows"))]
    let console_target = Target::Stdout; // Windows apps don't get a stderr handle
    #[cfg(debug_assertions)]
    let console = ConsoleAppender::builder()
        .target(console_target)
        .encoder(Box::new(PatternEncoder::new("[{d} {h({l:5})} {M}] {m}{n}")))
        .build();

    // Setup the file log roller
    let roller = FixedWindowRoller::builder()
        .build(
            archived_log_path.as_os_str().to_str().unwrap(),
            args.num_old_logs,
        )
        .unwrap();
    let file_policy = CompoundPolicy::new(
        Box::new(SizeTrigger::new(args.log_max_file_size)),
        Box::new(roller),
    );
    let file_appender = RollingFileAppender::builder()
        .append(true)
        .encoder(Box::new(PatternEncoder::new("[{d} {l:5} {M}] {m}{n}")))
        .build(log_path, Box::new(file_policy))
        .unwrap();

    // Setup the logging from all locations to use `LevelFilter::Error`
    let root = Root::builder().appender("file_appender");
    #[cfg(debug_assertions)]
    let root = root.appender("console");
    let root = root.build(LevelFilter::Error);

    // Setup the top level logging config
    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("file_appender", Box::new(file_appender)));

    #[cfg(debug_assertions)]
    let log_config = log_config.appender(Appender::builder().build("console", Box::new(console)));

    let log_config = log_config
        .logger(Logger::builder().build(APP_NAME, log_level)) // Setup the logging from the refbox app to use `log_level`
        .logger(Logger::builder().build("uwh_common", log_level)) // Setup the logging from mio to use `LevelFilter::Warn`
        .build(root)
        .unwrap();

    log4rs::init_config(log_config).unwrap();
    log_panics::init();

    let spacing = args.spacing.unwrap_or(args.scale / 4.0);

    let icon =
        icon::from_rgba(Vec::from(app_icon::DATA), app_icon::WIDTH, app_icon::HEIGHT).unwrap();

    if let Some(dir) = args.capture_previews {
        info!("Capturing front-display layout previews to {dir:?}");
        return sim_app::capture::run_capture(dir).map_err(|e| e.into());
    }

    if args.self_check {
        // Smoke test: logging + config are already initialised above. Prove we
        // can start on this machine without opening a window, spawning a sim
        // child, or grabbing hardware, then exit 0.
        info!("--self-check ok");
        return Ok(());
    }

    if args.is_simulator {
        let flags = sim_app::SimRefBoxAppFlags {
            tcp_port: args.binary_port,
            sunlight_mode: args.simulate_sunlight_display,
        };

        let window_settings = window::Settings {
            size: if args.simulate_sunlight_display {
                sim_app::sunlight_window_size(args.scale)
            } else {
                sim_app::matrix_window_size(args.scale, spacing)
            },
            position: window::Position::Specific(iced::Point::new(0.0, 40.0)),
            resizable: true,
            icon: Some(icon),
            ..Default::default()
        };

        info!("Starting Simulator UI");
        return iced::application(
            "Panel Simulator",
            sim_app::SimRefBoxApp::update,
            sim_app::SimRefBoxApp::view,
        )
        .subscription(sim_app::SimRefBoxApp::subscription)
        .window(window_settings)
        .style(sim_app::SimRefBoxApp::application_style)
        .font(include_bytes!("../resources/Roboto-Medium.ttf").as_slice())
        .run_with(|| sim_app::SimRefBoxApp::new(flags))
        .map_err(|e| e.into());
    } else {
        info!("Starting RefBox App");
    }

    let sim_spawn_config = SimSpawnConfig {
        binary_port: args.binary_port,
        json_port: args.json_port,
        scale: args.scale,
        spacing,
        sunlight_mode: args.simulate_sunlight_display,
        verbose: args.verbose,
        log_location: log_base_path.clone(),
        log_max_file_size: args.log_max_file_size,
        num_old_logs: args.num_old_logs,
    };

    let child = if args.no_simulate {
        None
    } else {
        info!(
            "Starting startup sim child with binary port {}",
            args.binary_port
        );
        match spawn_sim_child(&sim_spawn_config) {
            Ok(child) => Some(child),
            Err(e) => {
                error!("Failed to spawn startup simulator: {e:?}");
                None
            }
        }
    };

    let serial_ports = if let Some(port) = args.serial_port {
        info!(
            "Connection to serial port {port} with baud rate {}",
            args.baud_rate
        );
        let port = tokio_serial::new(port, args.baud_rate)
            .flow_control(FlowControl::None)
            .data_bits(DataBits::Eight)
            .parity(Parity::Even)
            .stop_bits(StopBits::One);
        vec![port]
    } else {
        vec![]
    };

    let config_path = confy::get_configuration_file_path(APP_NAME, None).unwrap();
    info!("Reading config file from {config_path:?}",);

    let config: Config = match confy::load(APP_NAME, None) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to use config file. Error: {e}");
            let config = match std::fs::read_to_string(&config_path) {
                Ok(file) => {
                    warn!("Found old config file, attempting migration");
                    match toml::from_str(&file) {
                        Ok(old_config) => Config::migrate(&old_config),
                        Err(e) => {
                            warn!("Failed to parse old config file. Error: {e}");
                            warn!("Using default config");
                            Config::default()
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read old config file. Error: {e}");
                    warn!("Using default config");
                    Config::default()
                }
            };
            confy::store(APP_NAME, None, &config).unwrap();
            config
        }
    };

    let window_size = Size::new(
        config.hardware.screen_x as f32,
        config.hardware.screen_y as f32,
    );

    // If a language was saved from the previous session and no CLI override was given, apply it.
    if LANGUAGE_OVERRIDE.lock().unwrap().is_none() {
        if let Some(lang) = config.language {
            *LANGUAGE_OVERRIDE.lock().unwrap() = Some(lang.as_lang_id());
        }
    }

    // Choose the default font based on the active language. Iced sets the font once at startup
    // and cannot change it at runtime, so a restart is required when switching between script
    // families (e.g. Latin ↔ Korean). The language select screen uses explicit per-button fonts
    // so all language names render correctly regardless of this default.
    let saved_language = config.language.unwrap_or(Language::English);
    let (default_font_family, default_font_weight) = match saved_language {
        Language::Korean | Language::Japanese | Language::Mandarin => {
            ("WenQuanYi Zen Hei", iced_core::font::Weight::Normal)
        }
        Language::Thai => ("Noto Sans Thai", iced_core::font::Weight::Normal),
        _ => ("Roboto", iced_core::font::Weight::Medium),
    };

    // The portal retry queue lives next to the config file. `config_path` is
    // the file itself (see above where it was loaded), so its parent is the
    // directory we want.
    let config_dir = config_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir);

    // Canonical path of the running binary, captured BEFORE any self-update swap.
    // The updater swaps the new binary in at this path and main() respawns it.
    let install_path = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(&p).ok());

    let mode = config.mode;
    let flags = app::RefBoxAppFlags {
        config,
        config_dir,
        serial_ports,
        binary_port: args.binary_port,
        json_port: args.json_port,
        sim_children: child.into_iter().collect(),
        sim_spawn_config,
        require_https: !args.allow_http,
        fullscreen: args.fullscreen,
        list_all_events: args.all_events,
        install_path: install_path.clone(),
        restart_argv: restart_argv.clone(),
    };

    // Roboto covers Latin scripts. The CJK subset covers Japanese, Korean, and Chinese
    // characters, and the Thai subset covers Thai. All fonts are bundled — no system font
    // install required. To regenerate subsets after updating translations, run:
    //   just regen-cjk-font
    let fonts = vec![
        Cow::from(&include_bytes!("../resources/Roboto-Medium.ttf")[..]),
        Cow::from(&include_bytes!("../resources/WqyZenHei-Subset.ttf")[..]),
        Cow::from(&include_bytes!("../resources/NotoSansThai-Subset.ttf")[..]),
    ];

    let settings = Settings {
        id: Some(APP_NAME.into()),
        fonts,
        default_font: Font {
            family: iced_core::font::Family::Name(default_font_family),
            weight: default_font_weight,
            stretch: iced_core::font::Stretch::Normal,
            style: iced_core::font::Style::Normal,
        },
        default_text_size: app::theme::SMALL_PLUS_TEXT.into(),
        antialiasing: false,
    };

    let window_settings = window::Settings {
        size: window_size,
        position: window::Position::Centered,
        resizable: false,
        icon: Some(icon),
        ..Default::default()
    };

    info!("Starting UI");
    let title = match mode {
        Mode::Rugby => "UWR Ref Box",
        Mode::Hockey6V6 | Mode::Hockey3V3 => "UWH Ref Box",
        Mode::BeepTest => "Beep Test",
    };
    let result = iced::application(title, app::RefBoxApp::update, app::RefBoxApp::view)
        .subscription(app::RefBoxApp::subscription)
        .settings(settings)
        .window(window_settings)
        .style(app::RefBoxApp::application_style)
        .run_with(|| app::RefBoxApp::new(flags));

    // If an in-app "Restart to Apply" path requested a restart, the iced
    // runtime has just finished closing all windows. Spawn a fresh copy of
    // the exe NOW (after the windows are gone) so the new instance opens on
    // a clean slate without overlapping the old windows. See
    // `app::RESTART_PENDING` for the trigger sites.
    if app::RESTART_PENDING.load(std::sync::atomic::Ordering::Relaxed) {
        // Prefer the install path captured BEFORE any self-update swap. After a
        // swap, current_exe() resolves to the old, now-unlinked inode and is
        // reported as "<path> (deleted)", which would fail to respawn. The
        // captured install_path still names the on-disk (new) binary. For a
        // normal (non-update) restart install_path equals the running binary, so
        // this is correct in both cases; current_exe() is only a last-resort
        // fallback if the path couldn't be captured at startup.
        match install_path
            .clone()
            .or_else(|| std::env::current_exe().ok())
        {
            Some(exe) => respawn(exe, &restart_argv),
            None => error!("Failed to locate exe for restart respawn"),
        }
    }

    result.map_err(|e| e.into())
}

#[cfg(test)]
mod restart_argv_tests {
    use super::*;

    fn argv_from(extra: &[&str]) -> Vec<String> {
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
    fn never_replays_language_is_simulator_or_capture_previews() {
        let argv = argv_from(&[
            "--language",
            "fr",
            "--is-simulator",
            "--capture-previews",
            "/tmp/previews",
        ]);
        assert!(!argv.contains(&"--language".to_string()));
        assert!(!argv.contains(&"--is-simulator".to_string()));
        assert!(!argv.contains(&"--capture-previews".to_string()));
    }

    #[test]
    fn replays_log_location_when_set_and_omits_when_unset() {
        let argv = argv_from(&["--log-location", "/tmp/test-logs"]);
        assert!(argv.contains(&"--log-location".to_string()));
        assert!(argv.contains(&"/tmp/test-logs".to_string()));
        assert!(!argv_from(&[]).contains(&"--log-location".to_string()));
    }

    #[test]
    fn never_replays_self_check() {
        assert!(!argv_from(&["--self-check"]).contains(&"--self-check".to_string()));
    }
}

#[cfg(test)]
mod sim_spawn_tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_config(verbose: u8, sunlight: bool) -> SimSpawnConfig {
        SimSpawnConfig {
            binary_port: 8001,
            json_port: 8000,
            scale: 4.0,
            spacing: 1.0,
            sunlight_mode: sunlight,
            verbose,
            log_location: PathBuf::from("/tmp/logs"),
            log_max_file_size: 5_000_000,
            num_old_logs: 3,
        }
    }

    #[test]
    fn argv_includes_required_flags() {
        let config = make_test_config(0, false);
        let argv = build_sim_argv(&config);
        assert!(argv.contains(&"--is-simulator".to_string()));
        assert!(argv.contains(&"--binary-port".to_string()));
        assert!(argv.contains(&"8001".to_string()));
    }

    #[test]
    fn argv_repeats_verbose_per_count() {
        let config = make_test_config(3, false);
        let argv = build_sim_argv(&config);
        assert_eq!(argv.iter().filter(|a| a.as_str() == "--verbose").count(), 3);
    }

    #[test]
    fn argv_includes_sunlight_flag_only_when_enabled() {
        let off = build_sim_argv(&make_test_config(0, false));
        let on = build_sim_argv(&make_test_config(0, true));
        assert!(!off.contains(&"--simulate-sunlight-display".to_string()));
        assert!(on.contains(&"--simulate-sunlight-display".to_string()));
    }
}
