use std::sync::mpsc::channel;
//use uwh_common::game_snapshot::GamePeriod;
use network::State;

use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
mod load_images;
mod network;
mod pages;

#[macroquad::main("UWH Overlay")]
async fn main() {
    let (tx, rx) = channel::<State>();
    std::thread::spawn(|| network::networking_thread(tx).unwrap());
    let args: Vec<String> = std::env::args().collect();
    let mut animaion_counter = 0f32;
    if args.len() != 2 {
        panic!("Got {} args instead of one", args.len() - 1);
    }

    let textures = if args[1] == String::from("--color") {
        load_images::Textures::init_color()
    } else if args[1] == String::from("--alpha") {
        load_images::Textures::init_alpha()
    } else {
        panic!("Expected --color or --alpha arg!")
    };

    let mut game_state: Option<State> = None;
    loop {
        clear_background(RED);
        if let Ok(state) = rx.try_recv() {
            game_state = Some(state);
        }

        if let Some(state) = &game_state {
            match state.snapshot.current_period {
                GamePeriod::BetweenGames => match state.snapshot.secs_in_period {
                    151..=u16::MAX => {
                        pages::next_game(&textures, &state);
                    }
                    30..=150 => {
                        pages::roster(&textures, &state, &mut animaion_counter);
                    }
                    _ => {
                        pages::pre_game_display(&textures);
                    }
                },
                _ => {
                    pages::final_scores(&textures);
                }
            }
        }
        next_frame().await
    }
}
