#![allow(clippy::useless_let_if_seq)]
use clap::Parser;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use iced::{Application, Settings};
use log::*;
use uwh_common::{config::Config, game_snapshot::*};
use uwh_matrix_drawing::*;

mod tournament_manager;

#[cfg(feature = "oldui")]
mod old_main;

#[cfg(not(feature = "oldui"))]
mod app;

const APP_CONFIG_NAME: &str = "uwh-refbox";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long)]
    /// Run the simulator GUI
    no_simulate: bool,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // This allows the use of error!(), warn!(), info!(), etc.
    env_logger::init();

    let _args = Cli::parse();

    let config: Config = confy::load(APP_CONFIG_NAME).unwrap();

    // If the user asks, simulate the display panels instead
    // TODO: fix sim and check for cli arg here
    if false {
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
            game_number: 0,
            next_game_number: 0,
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
        let window_size = (
            config.hardware.screen_x as u32,
            config.hardware.screen_y as u32,
        );
        let mut settings = Settings::with_flags(config);
        settings.window.size = window_size;
        settings.window.resizable = false;
        settings.default_text_size = app::style::NORMAL_TEXT;
        info!("Starting UI");
        app::RefBoxApp::run(settings)
    }?;

    Ok(())
}
