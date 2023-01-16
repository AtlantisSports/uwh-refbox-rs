#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::Parser;
use iced::{pure::Application, window::icon::Icon, Settings};
use log::*;
#[cfg(debug_assertions)]
use log4rs::append::console::ConsoleAppender;
use log4rs::{
    append::{
        console::Target,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

mod app;
mod app_icon;
mod penalty_editor;
mod sim_app;
mod sound_controller;
mod tournament_manager;

mod config;
use config::Config;

const APP_NAME: &str = "refbox";

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
    /// Don't require HTTPS to connect to uwhscores
    allow_http: bool,

    #[clap(long, short)]
    /// List all tournaments from uwhscores, including past ones
    all_tournaments: bool,

    #[clap(long)]
    /// Directory within which log files will be placed, default is platform dependent
    log_location: Option<PathBuf>,

    #[clap(long, default_value = "5000000")]
    /// Max size in bytes that a log file is allowed to reach before being rolled over
    log_max_file_size: u64,

    #[clap(long, default_value = "3")]
    /// Number of archived logs to keep
    num_old_logs: u32,

    #[clap(long, hide = true)]
    is_simulator: bool,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

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
        .build(root)
        .unwrap();

    log4rs::init_config(log_config).unwrap();
    log_panics::init();

    let spacing = args.spacing.unwrap_or(args.scale / 4.0);

    let icon =
        Icon::from_rgba(Vec::from(app_icon::DATA), app_icon::WIDTH, app_icon::HEIGHT).unwrap();

    if args.is_simulator {
        let flags = sim_app::SimRefBoxAppFlags {
            tcp_port: args.binary_port,
        };

        let mut settings = Settings::with_flags(flags);
        settings.window.size = sim_app::window_size(args.scale, spacing);
        settings.window.resizable = true;
        settings.window.icon = Some(icon);
        info!("Starting Simulator UI");
        <sim_app::SimRefBoxApp as iced::Application>::run(settings)?;

        return Ok(());
    } else {
        info!("Starting RefBox App");
    }

    let child = if args.no_simulate {
        None
    } else {
        let bin_name = std::env::current_exe()?.into_os_string();
        info!("Current bin_name: {bin_name:?}");

        let binary_port = args.binary_port.to_string();
        let json_port = args.json_port.to_string();
        let scale = args.scale.to_string();
        let spacing = spacing.to_string();
        let log_location = log_base_path.to_str().unwrap().to_string();
        let log_max_file_size = args.log_max_file_size.to_string();
        let num_old_logs = args.num_old_logs.to_string();

        let mut child_args = vec![
            "--is-simulator",
            "--binary-port",
            &binary_port,
            "--json-port",
            &json_port,
            "--scale",
            &scale,
            "--spacing",
            &spacing,
            "--log-location",
            &log_location,
            "--log-max-file-size",
            &log_max_file_size,
            "--num-old-logs",
            &num_old_logs,
        ];

        child_args.resize(child_args.len() + args.verbose as usize, "--verbose");

        debug!("Child args: {child_args:?}");

        info!("Starting child with birany port {binary_port}");
        let child = Command::new(bin_name)
            .args(child_args)
            .stdin(Stdio::null())
            .spawn()?;

        Some(child)
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

    info!(
        "Reading config file from {:?}",
        confy::get_configuration_file_path(APP_NAME, None).unwrap()
    );

    let config: Config = match confy::load(APP_NAME, None) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read config file, overwriting with default. Error: {e}");
            let config = Config::default();
            confy::store(APP_NAME, None, &config).unwrap();
            config
        }
    };

    if let Ok(offset) = time::UtcOffset::current_local_offset() {
        if offset != config.uwhscores.timezone {
            warn!(
                "The timezone in the config file ({}) does not match the detected system \
                 timezone ({offset}). The config timezone will be used.",
                config.uwhscores.timezone
            );
        }
    }

    let window_size = (
        config.hardware.screen_x as u32,
        config.hardware.screen_y as u32,
    );

    let flags = app::RefBoxAppFlags {
        config,
        serial_ports,
        binary_port: args.binary_port,
        json_port: args.json_port,
        sim_child: child,
        require_https: !args.allow_http,
        fullscreen: args.fullscreen,
        list_all_tournaments: args.all_tournaments,
    };

    let mut settings = Settings::with_flags(flags);
    settings.window.size = window_size;
    settings.window.resizable = false;
    settings.window.icon = Some(icon);
    settings.default_text_size = app::style::SMALL_PLUS_TEXT;
    settings.default_font = Some(include_bytes!("../resources/Roboto-Medium.ttf"));
    info!("Starting UI");
    app::RefBoxApp::run(settings)?;

    Ok(())
}
