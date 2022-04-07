#![allow(clippy::useless_let_if_seq)]
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    SubCommand,
};
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use iced::{Application, Settings};
use log::*;
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Write},
};
use uwh_common::{config::Config, game_snapshot::*};
use uwh_matrix_drawing::*;

mod tournament_manager;

#[cfg(feature = "oldui")]
mod old_main;

#[cfg(not(feature = "oldui"))]
mod app;
#[cfg(not(feature = "oldui"))]
mod style;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // This allows the use of error!(), warn!(), info!(), etc.
    env_logger::init();

    // Parse argumanets given on the command line
    let matches = app_from_crate!()
        .setting(AppSettings::VersionlessSubcommands)
        .arg(
            Arg::with_name("config-file")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("PATH")
                .help("Path to the config file"),
        )
        .subcommand(
            SubCommand::with_name("config")
                .about("Manipulate the config file")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("check").about("Check that the config file is valid"),
                )
                .subcommand(
                    SubCommand::with_name("generate").about("Generate a default config file"),
                ),
        )
        .subcommand(
            SubCommand::with_name("simulate").about("Run the simulation of the display panels"),
        )
        .get_matches();

    let config_path = matches.value_of("config-file").unwrap_or("timeshark.toml");

    // If the `config` command was given, handle that then exit
    if let Some(matches) = matches.subcommand_matches("config") {
        if let Some(_matches) = matches.subcommand_matches("check") {
            match Config::new_from_file(config_path) {
                Ok(_) => println!("Config file {} is valid", config_path),
                Err(_) => println!("Config file {} is not valid :(", config_path),
            }
            return Ok(());
        } else if let Some(_matches) = matches.subcommand_matches("generate") {
            let mut file = match create_new_file(config_path) {
                Ok(f) => f,
                Err(e) => match e.kind() {
                    ErrorKind::AlreadyExists => {
                        let mut backup_path = config_path.to_string();
                        backup_path.push_str(".bak");
                        error!(
                            "Warning: {} already exists, moving it to {}",
                            config_path, &backup_path
                        );
                        let _backup_file = create_new_file(&backup_path)?;
                        std::fs::rename(config_path, &backup_path)?;
                        create_new_file(config_path)?
                    }
                    _ => {
                        error!("Error: could not open {} for writing: {}", config_path, e);
                        return Err(Box::new(e));
                    }
                },
            };

            let config: Config = Default::default();
            let config_str = toml::ser::to_string(&config)?;
            file.write_all(&config_str.into_bytes())?;

            println!("Wrote default config to {}", config_path);
            return Ok(());
        }
    }

    let config = Config::new_from_file(config_path)?;

    // If the user asks, simulate the display panels instead
    if matches.subcommand_matches("simulate").is_some() {
        // Make a fake game state
        let state = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 754, // 12:34
            timeout: TimeoutSnapshot::None,
            b_score: 1,
            w_score: 5,
            b_penalties: vec![PenaltySnapshot {
                player_number: 3,
                time: PenaltyTime::Seconds(74),
            }],
            w_penalties: vec![PenaltySnapshot {
                player_number: 5,
                time: PenaltyTime::Seconds(32),
            }],
        };

        let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(256, 64));

        let output_settings = OutputSettingsBuilder::new()
            .scale(3)
            .pixel_spacing(1)
            .build();

        draw_panels(&mut display, state.into(), config.hardware.white_on_right).unwrap();

        let mut window = Window::new("Panel Simulator", &output_settings);

        window.show_static(&display);

        return Ok(());
    }

    #[cfg(feature = "oldui")]
    old_main::old_main(config)?;

    #[cfg(not(feature = "oldui"))]
    {
        let mut settings = Settings::with_flags(config.game);
        settings.window.size = (
            config.hardware.screen_x as u32,
            config.hardware.screen_y as u32,
        );
        settings.window.resizable = false;
        settings.default_text_size = crate::style::NORMAL_TEXT;
        info!("Starting UI");
        app::RefBoxApp::run(settings)
    }?;

    Ok(())
}

fn create_new_file(path: &str) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}
