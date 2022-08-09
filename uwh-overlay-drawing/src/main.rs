use std::sync::mpsc::channel;
//use uwh_common::game_snapshot::GamePeriod;
use network::State;

use macroquad::prelude::*;
use uwh_common::{config::Game, game_snapshot::GamePeriod};
mod load_images;
mod network;
mod pages_alpha;
mod pages_color;

#[macroquad::main("UWH Overlay")]
async fn main() {
    let (tx, rx) = channel::<State>();
    std::thread::spawn(|| network::networking_thread(tx).unwrap());
    let args: Vec<String> = std::env::args().collect();
    let mut animaion_counter = 0f32;
    assert!(
        args.len() == 2,
        "Got {} args instead of one",
        args.len() - 1
    );
    let (textures, is_alpha_mode) = if args[1] == *"--color" {
        (load_images::Textures::init_color(), false)
    } else if args[1] == *"--alpha" {
        (load_images::Textures::init_alpha(), true)
    } else {
        panic!("Expected --color or --alpha arg!")
    };

    let mut game_state: Option<State> = None;
    loop {
        clear_background(BLACK);
        if let Ok(state) = rx.try_recv() {
            game_state = Some(state);
        }

        if let Some(state) = &game_state {
            match state.snapshot.current_period {
                GamePeriod::BetweenGames => match state.snapshot.secs_in_period {
                    151..=u16::MAX => {
                        if !is_alpha_mode {
                            pages_color::next_game(&textures, state);
                        } else {
                            pages_alpha::next_game(&textures, state);
                        }
                    }
                    30..=150 => {
                        if !is_alpha_mode {
                            pages_color::roster(&textures, state, &mut animaion_counter);
                        } else {
                            pages_alpha::roster(&textures, state, &mut animaion_counter);
                        }
                    }
                    _ => {
                        if !is_alpha_mode {
                            pages_color::pre_game_display(&textures, state);
                        } else {
                            pages_alpha::pre_game_display(&textures, state, &mut animaion_counter);
                        }
                    }
                },
                GamePeriod::FirstHalf
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeSecondHalf => {
                    if !is_alpha_mode {
                        pages_color::in_game_display(&textures, state, &mut animaion_counter);
                    } else {
                        pages_alpha::in_game_display(&textures, state, &mut animaion_counter);
                    }
                }
                GamePeriod::HalfTime | GamePeriod::OvertimeHalfTime => {
                    if !is_alpha_mode {
                        pages_color::half_time_display(&textures);
                    } else {
                        pages_alpha::half_time_display(&textures);
                    }
                }
                _ => {
                    if !is_alpha_mode {
                        pages_color::final_scores(&textures);
                    } else {
                        pages_alpha::final_scores(&textures);
                    }
                }
            }
        }
        next_frame().await;
    }
}
