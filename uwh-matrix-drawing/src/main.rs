#![cfg(feature = "std")]

use clap::{ArgGroup, Args, Parser, Subcommand};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use serialport::{self, DataBits, FlowControl, Parity, StopBits};
use std::{fs, path::PathBuf, thread, time::Duration};
use uwh_common::game_snapshot::*;
use uwh_matrix_drawing::*;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(long)]
    /// Run the simulator GUI
    simulate: bool,

    #[clap(
        long,
        value_name = "SERIAL_PORT",
        default_missing_value = "/dev/ttyAMA0"
    )]
    /// Serialize the states and send them over the provided port
    hardware: Option<String>,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Use a state file instead of automatically generating states
    State(StateFile),
}

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("state_file").required(true).args(&["file", "generate"])))]
struct StateFile {
    #[clap(value_name = "PATH")]
    /// File to read state information from
    file: Option<PathBuf>,

    #[clap(long, value_name = "PATH", default_missing_value = "state.json")]
    /// Location to generate a sample state file in
    generate: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let long_time_cycle = [900, 743, 154, 60, 59, 58, 7, 6, 5, 4, 3, 2, 1, 0];
    let medium_time_cycle = [180, 154, 60, 59, 58, 7, 6, 5, 4, 3, 2, 1, 0];
    let short_time_cycle = [60, 59, 58, 7, 6, 5, 4, 3, 2, 1, 0];

    let timeout_cycle = [
        TimeoutSnapshot::Black(60),
        TimeoutSnapshot::Black(59),
        TimeoutSnapshot::Black(58),
        TimeoutSnapshot::Black(7),
        TimeoutSnapshot::Black(6),
        TimeoutSnapshot::Black(5),
        TimeoutSnapshot::Black(4),
        TimeoutSnapshot::Black(3),
        TimeoutSnapshot::Black(2),
        TimeoutSnapshot::Black(1),
        TimeoutSnapshot::Black(0),
        TimeoutSnapshot::White(60),
        TimeoutSnapshot::White(59),
        TimeoutSnapshot::White(58),
        TimeoutSnapshot::White(7),
        TimeoutSnapshot::White(6),
        TimeoutSnapshot::White(5),
        TimeoutSnapshot::White(4),
        TimeoutSnapshot::White(3),
        TimeoutSnapshot::White(2),
        TimeoutSnapshot::White(1),
        TimeoutSnapshot::White(0),
        TimeoutSnapshot::Ref(0),
        TimeoutSnapshot::Ref(1),
        TimeoutSnapshot::Ref(2),
        TimeoutSnapshot::Ref(3),
        TimeoutSnapshot::Ref(4),
        TimeoutSnapshot::Ref(5),
        TimeoutSnapshot::Ref(6),
        TimeoutSnapshot::Ref(7),
        TimeoutSnapshot::Ref(58),
        TimeoutSnapshot::Ref(59),
        TimeoutSnapshot::Ref(60),
        TimeoutSnapshot::PenaltyShot(0),
        TimeoutSnapshot::PenaltyShot(1),
        TimeoutSnapshot::PenaltyShot(2),
        TimeoutSnapshot::PenaltyShot(3),
        TimeoutSnapshot::PenaltyShot(4),
        TimeoutSnapshot::PenaltyShot(5),
        TimeoutSnapshot::PenaltyShot(6),
        TimeoutSnapshot::PenaltyShot(7),
        TimeoutSnapshot::PenaltyShot(58),
        TimeoutSnapshot::PenaltyShot(59),
        TimeoutSnapshot::PenaltyShot(60),
        TimeoutSnapshot::None,
    ];

    let period_cycle = [
        GamePeriod::BetweenGames,
        GamePeriod::FirstHalf,
        GamePeriod::HalfTime,
        GamePeriod::SecondHalf,
        GamePeriod::PreOvertime,
        GamePeriod::OvertimeFirstHalf,
        GamePeriod::OvertimeHalfTime,
        GamePeriod::OvertimeSecondHalf,
        GamePeriod::PreSuddenDeath,
        GamePeriod::SuddenDeath,
    ];

    let score_loop_count: u8 = 0;

    let repeat_state = if let Some(Commands::State(state_args)) = args.command {
        if let Some(path) = state_args.generate {
            let state = GameSnapshot {
                current_period: GamePeriod::FirstHalf,
                secs_in_period: 900,
                timeout: TimeoutSnapshot::White(36),
                b_score: 2,
                w_score: 6,
                b_penalties: vec![
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(66),
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::Seconds(21),
                    },
                    PenaltySnapshot {
                        player_number: 13,
                        time: PenaltyTime::TotalDismissal,
                    },
                ],
                w_penalties: vec![
                    PenaltySnapshot {
                        player_number: 1,
                        time: PenaltyTime::Seconds(300),
                    },
                    PenaltySnapshot {
                        player_number: 5,
                        time: PenaltyTime::Seconds(50),
                    },
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::TotalDismissal,
                    },
                ],
            };

            let to_save = serde_json::to_vec_pretty(&state)?;
            fs::write(&path, to_save)?;

            let state: GameSnapshotNoHeap = state.into();

            let len = bincode::serialize(&state)?.len();

            println!(
                "The encoded length of the state (as would be sent to the panels) was {len} bytes"
            );

            return Ok(());
        } else {
            let state: GameSnapshot =
                serde_json::from_slice(&fs::read(&state_args.file.unwrap())?)?;
            Some(state)
        }
    } else {
        None
    };

    let state_iter: Box<dyn Iterator<Item = GameSnapshot>> = if let Some(state) = repeat_state {
        Box::new([state].into_iter().cycle())
    } else {
        Box::new(
            period_cycle
                .into_iter()
                .cycle()
                .flat_map(
                    |period| -> Box<dyn Iterator<Item = ((GamePeriod, TimeoutSnapshot), u16)>> {
                        match period {
                            GamePeriod::FirstHalf
                            | GamePeriod::SecondHalf
                            | GamePeriod::SuddenDeath => Box::new(
                                [period]
                                    .into_iter()
                                    .cycle()
                                    .zip(timeout_cycle.into_iter())
                                    .zip([long_time_cycle[0]].into_iter().cycle())
                                    .chain(
                                        [period]
                                            .into_iter()
                                            .cycle()
                                            .zip(
                                                [*timeout_cycle.last().unwrap()]
                                                    .into_iter()
                                                    .cycle(),
                                            )
                                            .zip(long_time_cycle.into_iter()),
                                    ),
                            ),
                            GamePeriod::OvertimeFirstHalf | GamePeriod::OvertimeSecondHalf => {
                                Box::new(
                                    [period]
                                        .into_iter()
                                        .cycle()
                                        .zip(timeout_cycle.into_iter())
                                        .zip([medium_time_cycle[0]].into_iter().cycle())
                                        .chain(
                                            [period]
                                                .into_iter()
                                                .cycle()
                                                .zip(
                                                    [*timeout_cycle.last().unwrap()]
                                                        .into_iter()
                                                        .cycle(),
                                                )
                                                .zip(medium_time_cycle.into_iter()),
                                        ),
                                )
                            }
                            GamePeriod::HalfTime | GamePeriod::BetweenGames => Box::new(
                                [period]
                                    .into_iter()
                                    .cycle()
                                    .zip([*timeout_cycle.last().unwrap()].into_iter().cycle())
                                    .zip(medium_time_cycle.into_iter()),
                            ),
                            GamePeriod::PreOvertime
                            | GamePeriod::OvertimeHalfTime
                            | GamePeriod::PreSuddenDeath => Box::new(
                                [period]
                                    .into_iter()
                                    .cycle()
                                    .zip([*timeout_cycle.last().unwrap()].into_iter().cycle())
                                    .zip(short_time_cycle.into_iter()),
                            ),
                        }
                    },
                )
                .scan(
                    score_loop_count,
                    |score_loop_count, ((current_period, timeout), secs_in_period)| {
                        let b_score;
                        let w_score;
                        let b_penalties;
                        let w_penalties;

                        match *score_loop_count {
                            0..=4 => {
                                b_score = 4;
                                w_score = 9;
                            }
                            5..=9 => {
                                b_score = 11;
                                w_score = 24;
                            }
                            10..=14 => {
                                b_score = 2;
                                w_score = 5;
                            }
                            _ => {
                                b_score = 13;
                                w_score = 27;
                            }
                        };

                        match *score_loop_count {
                            0..=9 => {
                                b_penalties = vec![];
                                w_penalties = vec![];
                            }
                            _ => {
                                b_penalties = vec![
                                    PenaltySnapshot {
                                        player_number: 4,
                                        time: PenaltyTime::Seconds(66),
                                    },
                                    PenaltySnapshot {
                                        player_number: 7,
                                        time: PenaltyTime::Seconds(21),
                                    },
                                    PenaltySnapshot {
                                        player_number: 13,
                                        time: PenaltyTime::TotalDismissal,
                                    },
                                ];
                                w_penalties = vec![
                                    PenaltySnapshot {
                                        player_number: 1,
                                        time: PenaltyTime::Seconds(300),
                                    },
                                    PenaltySnapshot {
                                        player_number: 5,
                                        time: PenaltyTime::Seconds(50),
                                    },
                                    PenaltySnapshot {
                                        player_number: 2,
                                        time: PenaltyTime::TotalDismissal,
                                    },
                                ];
                            }
                        };

                        *score_loop_count = (*score_loop_count + 1) % 20;

                        Some(GameSnapshot {
                            current_period,
                            secs_in_period,
                            timeout,
                            b_score,
                            w_score,
                            b_penalties,
                            w_penalties,
                        })
                    },
                ),
        )
    };

    let simulate = if args.hardware.is_some() {
        args.simulate
    } else {
        true
    };

    let mut serial = if let Some(path) = args.hardware {
        Some(
            serialport::new(path, 115200)
                .data_bits(DataBits::Eight)
                .flow_control(FlowControl::None)
                .parity(Parity::Even)
                .stop_bits(StopBits::One)
                .open()?,
        )
    } else {
        None
    };

    let output_settings = OutputSettingsBuilder::new()
        .scale(4)
        .pixel_spacing(1)
        .build();

    let mut simulation = if simulate {
        let display = SimulatorDisplay::<Rgb888>::new(Size::new(256, 64));
        let window = Window::new("Panel Simulator", &output_settings);
        Some((display, window))
    } else {
        None
    };

    if simulation.is_none() {
        println!("Press Ctrl-C to stop");
    }

    'running: for state in state_iter {
        let state: GameSnapshotNoHeap = state.into();

        if let Some(port) = serial.as_mut() {
            let to_send = bincode::serialize(&state)?;
            port.write_all(&to_send)?;
        }

        if let Some((display, window)) = simulation.as_mut() {
            display.clear(Rgb888::BLACK).unwrap();

            Rectangle::new(Point::new(0, 0), Size::new(64, 64))
                .into_styled(PrimitiveStyle::with_fill(Rgb888::CSS_DIM_GRAY))
                .draw(display)
                .unwrap();
            Rectangle::new(Point::new(192, 0), Size::new(64, 64))
                .into_styled(PrimitiveStyle::with_fill(Rgb888::CSS_DIM_GRAY))
                .draw(display)
                .unwrap();

            draw_panels(display, state, true).unwrap();

            window.update(display);

            if window.events().any(|e| e == SimulatorEvent::Quit) {
                break 'running;
            }
        }
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
