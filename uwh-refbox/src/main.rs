use clap::Parser;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use iced::{pure::Application, Settings};
use log::*;
use std::{
    io::{ErrorKind, Read},
    net::TcpStream,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};
use uwh_common::config::Config;
use uwh_matrix_drawing::{transmitted_data::TransmittedData, *};

mod penalty_editor;
mod tournament_manager;

mod app;

const APP_CONFIG_NAME: &str = "uwh-refbox";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, short)]
    /// Don't run the simulator GUI
    no_simulate: bool,

    #[clap(long, short, default_value = "4")]
    /// Scale argument for the simulator
    scale: u32,

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

    #[clap(long, hide = true)]
    is_simulator: bool,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Cli::parse();

    info!("Parsed arguments");

    if args.is_simulator {
        let mut stream;
        let start_time = Instant::now();
        loop {
            match TcpStream::connect(("localhost", args.binary_port)) {
                Ok(conn) => {
                    stream = conn;
                    break;
                }
                Err(e) => {
                    if e.kind() == ErrorKind::ConnectionRefused {
                        if Instant::now().duration_since(start_time) > Duration::from_secs(5) {
                            error!("Couldn't connect to refbox within 5 seconds");
                            Err(std::io::Error::new(
                                ErrorKind::TimedOut,
                                "Connection to Refbox Failed",
                            ))?;
                        } else {
                            continue;
                        }
                    } else {
                        Err(e)?;
                    }
                }
            }
        }

        stream.set_read_timeout(None)?;

        let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(256, 64));

        let output_settings = OutputSettingsBuilder::new()
            .scale(args.scale)
            .pixel_spacing(args.scale.saturating_sub(1) / 4 + 1)
            .build();

        let mut window = Window::new("Panel Simulator", &output_settings);

        let mut buffer = [0u8; TransmittedData::ENCODED_LEN];

        let mut error_count = 0;
        loop {
            let state = match stream.read(&mut buffer) {
                Err(e) => {
                    error!("Error reading stream: {}", e);
                    error_count += 1;
                    if error_count > 4 {
                        break;
                    } else {
                        continue;
                    }
                }
                Ok(val) => match val {
                    TransmittedData::ENCODED_LEN => {
                        error_count = 0;
                        //info!("Received data: {buffer:?}");
                        TransmittedData::decode(&buffer)?
                    }
                    other => {
                        error!("Received message wrong length: {other}");
                        thread::sleep(Duration::from_millis(50));
                        error_count += 1;
                        if error_count > 4 {
                            break;
                        } else {
                            continue;
                        }
                    }
                },
            };

            //info!("Decoded data: {state:?}");
            display.clear(Rgb888::BLACK)?;
            draw_panels(&mut display, state.snapshot, state.white_on_right).unwrap();

            window.update(&display);

            if window.events().any(|e| e == SimulatorEvent::Quit) {
                break;
            }
        }

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

    let config: Config = confy::load(APP_CONFIG_NAME, None).unwrap();

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
    };

    let mut settings = Settings::with_flags(flags);
    settings.window.size = window_size;
    settings.window.resizable = false;
    settings.default_text_size = app::style::SMALL_PLUS_TEXT;
    settings.default_font = Some(include_bytes!("../Roboto-Medium.ttf"));
    info!("Starting UI");
    app::RefBoxApp::run(settings)?;

    Ok(())
}
