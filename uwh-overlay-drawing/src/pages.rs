use crate::{load_images::Textures, network};
use macroquad::prelude::*;

fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin().read_line(&mut buffer).expect("Failed");
    buffer.trim().parse::<T>().unwrap_or(Default::default())
}

pub fn roster(textures: &Textures, state: &network::State) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, -650f32, WHITE);
    draw_texture(*textures.team_black_graphic(), 1090f32, 220f32, WHITE);
    draw_texture(*textures.team_white_graphic(), 150f32, 220f32, WHITE);
    draw_texture(
        *textures.team_white_graphic(),
        150f32,
        220f32 + 60f32,
        WHITE,
    );
    draw_texture(
        *textures.team_black_graphic(),
        1090f32,
        220f32 + 60f32,
        WHITE,
    );
    draw_text_ex(
        state.white.to_uppercase().as_str(),
        340f32,
        155f32,
        TextParams {
            font: textures.font(),
            font_size: 50,
            color: BLACK,
            ..Default::default()
        },
    );
    draw_text_ex(
        state.black.to_uppercase().as_str(),
        1240f32,
        155f32,
        TextParams {
            font: textures.font(),
            font_size: 45,
            ..Default::default()
        },
    );
    let min = state.snapshot.secs_in_period / 60;
    let secs = state.snapshot.secs_in_period % 60;
    draw_text_ex(
        format!("{}:{}", min, secs).as_str(),
        923f32,
        1020f32,
        TextParams {
            font: textures.font(),
            font_size: 50,
            ..Default::default()
        },
    );
    draw_text_ex(
        "NEXT GAME",
        905f32,
        1044f32,
        TextParams {
            font: textures.font(),
            font_size: 20,
            ..Default::default()
        },
    );
}

pub fn next_game(textures: &Textures, state: &network::State) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, 0f32, WHITE);
    draw_text_ex(
        state.white.to_uppercase().as_str(),
        340f32,
        805f32,
        TextParams {
            font: textures.font(),
            font_size: 50,
            color: BLACK,
            ..Default::default()
        },
    );
    draw_text_ex(
        state.black.to_uppercase().as_str(),
        1240f32,
        805f32,
        TextParams {
            font: textures.font(),
            font_size: 45,
            ..Default::default()
        },
    );
    let min = state.snapshot.secs_in_period / 60;
    let secs = state.snapshot.secs_in_period % 60;
    draw_text_ex(
        format!("{}:{}", min, secs).as_str(),
        923f32,
        1020f32,
        TextParams {
            font: textures.font(),
            font_size: 50,
            ..Default::default()
        },
    );
    draw_text_ex(
        "NEXT GAME",
        905f32,
        1044f32,
        TextParams {
            font: textures.font(),
            font_size: 20,
            ..Default::default()
        },
    );
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
