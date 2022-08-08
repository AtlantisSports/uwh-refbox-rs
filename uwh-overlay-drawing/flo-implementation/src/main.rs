use std::sync::mpsc::channel;
//use uwh_common::game_snapshot::GamePeriod;
use uwh_common::game_snapshot::GameSnapshot;

use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
mod load_images;
mod network;
mod pages;

#[macroquad::main("UWH Overlay")]
async fn main() {
    let (tx, rx) = channel::<GameSnapshot>();
    std::thread::spawn(|| network::networking_thread(tx).unwrap());
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        panic!("Got {} args instead of one", args.len());
    }

    let textures = if args[1] == String::from("--color") {
        load_images::Textures::init_color()
    } else {
        load_images::Textures::init_alpha()
    };
    let font = load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF")).unwrap();

    let mut game_state: Option<GameSnapshot> = None;
    loop {
        clear_background(Color::default());
        if let Ok(state) = rx.try_recv() {
            game_state = Some(state);
        }

        draw_text_ex(
            "Custom font size:",
            50.0,
            50.0,
            TextParams {
                font,
                font_size: 50,
                ..Default::default()
            },
        );
        if let Some(state) = &game_state {
            match state.current_period {
                GamePeriod::BetweenGames => match state.secs_in_period {
                    121..=u32::MAX => {
                        pages::next_game(&textures);

                        //pages::next_game(&alpha_textures);
                    }
                    30..=120 => {
                        pages::roster(&textures);

                        //pages::roster(&alpha_textures);
                    }
                    _ => {
                        pages::pre_game_display(&textures);

                        //pages::pre_game_display(&alpha_textures);
                    }
                },
                _ => {
                    pages::final_scores(&textures);

                    //pages::final_scores(&alpha_textures);
                }
            }
        }
        next_frame().await
    }
}
