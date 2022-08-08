use crate::load_images::Textures;
use macroquad::prelude::*;

fn _get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin().read_line(&mut buffer).expect("Failed");
    buffer.trim().parse::<T>().unwrap_or(Default::default())
}

pub fn roster(textures: &Textures) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_black_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_black_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_white_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_white_graphic(), 0_f32, 0f32, WHITE);
}

pub fn next_game(textures: &Textures) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, 0f32, WHITE);
}

pub fn final_scores(textures: &Textures) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.final_score_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, 0f32, WHITE);
}

pub fn pre_game_display(textures: &Textures) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
}
