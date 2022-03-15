#![cfg(feature = "std")]

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use std::{thread, time::Duration};
use uwh_common::game_snapshot::*;
use uwh_matrix_drawing::*;

fn main() {
    let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(256, 64));

    let output_settings = OutputSettingsBuilder::new()
        .scale(4)
        .pixel_spacing(1)
        .build();

    let mut state = GameSnapshot {
        current_period: GamePeriod::PreOvertime,
        secs_in_period: 901,
        timeout: TimeoutSnapshot::None,
        b_score: 0,
        w_score: 0,
        b_penalties: vec![],
        w_penalties: vec![],
    };

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

    let mut window = Window::new("Panel Simulator", &output_settings);

    let mut loop_count = 0;

    'running: loop {
        let mut do_update = |state: &mut GameSnapshot| {
            display.clear(Rgb888::BLACK).unwrap();

            Rectangle::new(Point::new(0, 0), Size::new(64, 64))
                .into_styled(PrimitiveStyle::with_fill(Rgb888::CSS_DIM_GRAY))
                .draw(&mut display)
                .unwrap();
            Rectangle::new(Point::new(192, 0), Size::new(64, 64))
                .into_styled(PrimitiveStyle::with_fill(Rgb888::CSS_DIM_GRAY))
                .draw(&mut display)
                .unwrap();

            match loop_count {
                0..=4 => {
                    state.b_score = 4;
                    state.w_score = 9;
                    state.b_penalties = vec![];
                    state.w_penalties = vec![];
                }
                5..=9 => {
                    state.b_score = 11;
                    state.w_score = 24;
                    state.b_penalties = vec![];
                    state.w_penalties = vec![];
                }
                10..=14 => {
                    state.b_score = 2;
                    state.w_score = 5;
                    state.b_penalties = vec![
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
                    state.w_penalties = vec![
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
                _ => {
                    state.b_score = 11;
                    state.w_score = 24;
                    state.b_penalties = vec![
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
                    state.w_penalties = vec![
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

            loop_count = (loop_count + 1) % 20;

            draw_panels(&mut display, state.clone().into(), true).unwrap();

            window.update(&display);

            if window.events().any(|e| e == SimulatorEvent::Quit) {
                return true;
            }
            thread::sleep(Duration::from_secs(1));
            false
        };

        state.current_period = if let Some(per) = state.current_period.next_period() {
            per
        } else {
            GamePeriod::BetweenGames
        };

        match state.current_period {
            GamePeriod::FirstHalf | GamePeriod::SecondHalf | GamePeriod::SuddenDeath => {
                state.secs_in_period = long_time_cycle[0];
                if do_update(&mut state) {
                    break 'running;
                }

                for timeout in timeout_cycle.iter() {
                    state.timeout = *timeout;
                    if do_update(&mut state) {
                        break 'running;
                    }
                }

                for time in long_time_cycle.iter() {
                    state.secs_in_period = *time;
                    if do_update(&mut state) {
                        break 'running;
                    }
                }
            }
            GamePeriod::OvertimeFirstHalf | GamePeriod::OvertimeSecondHalf => {
                state.secs_in_period = medium_time_cycle[0];
                if do_update(&mut state) {
                    break 'running;
                }

                for timeout in timeout_cycle.iter() {
                    state.timeout = *timeout;
                    if do_update(&mut state) {
                        break 'running;
                    }
                }

                for time in medium_time_cycle.iter() {
                    state.secs_in_period = *time;
                    if do_update(&mut state) {
                        break 'running;
                    }
                }
            }
            GamePeriod::HalfTime | GamePeriod::BetweenGames => {
                for time in medium_time_cycle.iter() {
                    state.secs_in_period = *time;
                    if do_update(&mut state) {
                        break 'running;
                    }
                }
            }
            GamePeriod::PreOvertime | GamePeriod::OvertimeHalfTime | GamePeriod::PreSuddenDeath => {
                for time in short_time_cycle.iter() {
                    state.secs_in_period = *time;
                    if do_update(&mut state) {
                        break 'running;
                    }
                }
            }
        };
    }
}
