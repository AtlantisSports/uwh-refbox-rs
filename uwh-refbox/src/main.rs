#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::Parser;
use iced::{pure::Application, window::icon::Icon, Settings};
use log::*;
use std::process::{Command, Stdio};
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};
use uwh_common::{config::Config, uwhscores};

mod penalty_editor;
mod tournament_manager;

mod app;
mod app_icon;
mod sim_app;

const APP_CONFIG_NAME: &str = "uwh-refbox";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, short)]
    /// Don't run the simulator GUI
    no_simulate: bool,

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

    #[clap(long, hide = true)]
    is_simulator: bool,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Cli::parse();

    info!("Parsed arguments");

    let spacing = args.spacing.unwrap_or(args.scale / 4.0);

    let icon =
        Icon::from_rgba(Vec::from(app_icon::DATA), app_icon::WIDTH, app_icon::HEIGHT).unwrap();

    if args.is_simulator {
        let flags = sim_app::SimRefBoxAppFlags {
            tcp_port: args.binary_port,
            scale: args.scale,
            spacing,
        };

        let mut settings = Settings::with_flags(flags);
        settings.window.size = sim_app::window_size(args.scale, spacing);
        settings.window.resizable = false;
        settings.window.icon = Some(icon);
        info!("Starting Simulator UI");
        <sim_app::SimRefBoxApp as iced::Application>::run(settings)?;

        return Ok(());
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

        info!("Starting child with birany port {binary_port}");
        let child = Command::new(bin_name)
            .args([
                "--is-simulator",
                "--binary-port",
                &binary_port,
                "--json-port",
                &json_port,
                "--scale",
                &scale,
                "--spacing",
                &spacing,
            ])
            .stdin(Stdio::null())
            .spawn()?;

        Some(child)
    };

    let serial_ports = if let Some(port) = args.serial_port {
        info!(
            "Connection to serail port {port}  with baud rate {}",
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
        confy::get_configuration_file_path(APP_CONFIG_NAME, None).unwrap()
    );

    let config: Config = match confy::load(APP_CONFIG_NAME, None) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read config file, overwriting with default. Error: {e}");
            let config = Config::default();
            confy::store(APP_CONFIG_NAME, None, &config).unwrap();
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
