use clap::Parser;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use iced::{Application, Settings};
use log::*;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    thread,
    time::Duration,
};
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

    #[clap(long, hide = true)]
    port: Option<u16>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Cli::parse();

    info!("Parsed arguments");

    if let Some(port) = args.port {
        let mut stream = TcpStream::connect(("localhost", port))?;

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

            display.clear(Rgb888::BLACK)?;
            draw_panels(&mut display, state.snapshot, state.white_on_right).unwrap();

            window.update(&display);

            if window.events().any(|e| e == SimulatorEvent::Quit) {
                break;
            }
        }

        return Ok(());
    }

    let (stream, child) = if args.no_simulate {
        (None, None)
    } else {
        let bin_name = std::env::current_exe()?.into_os_string();
        info!("Current bin_name: {bin_name:?}");

        let listener = TcpListener::bind(("localhost", 0))?;
        let port = listener.local_addr()?.port().to_string();

        info!("Starting child with port arg {port}");
        let child = Command::new(bin_name)
            .args(["--port", &port, "--scale", &args.scale.to_string()])
            .stdin(Stdio::null())
            .spawn()?;

        info!("Waiting for connection from simulator");
        let (stream, addr) = listener.accept()?;
        info!("Got connection from {addr}");

        (Some(stream), Some(child))
    };

    let config: Config = confy::load(APP_CONFIG_NAME).unwrap();

    let window_size = (
        config.hardware.screen_x as u32,
        config.hardware.screen_y as u32,
    );
    let mut settings = Settings::with_flags((config, stream, child));
    settings.window.size = window_size;
    settings.window.resizable = false;
    settings.default_text_size = app::style::SMALL_PLUS_TEXT;
    settings.default_font = Some(include_bytes!("../Roboto-Medium.ttf"));
    info!("Starting UI");
    app::RefBoxApp::run(settings)?;

    Ok(())
}
