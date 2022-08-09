use crate::{
    load_images::Textures,
    network::{self, State},
};
use macroquad::prelude::*;

trait Interpolate {
    /// value must be a floater varying from 0 to 1, denoting the lowest to highest limits of the range
    fn interpolate_linear(&self, value: f32) -> f32;
}

impl Interpolate for (f32, f32) {
    fn interpolate_linear(&self, value: f32) -> f32 {
        (self.1 - self.0).mul_add(value, self.0)
    }
}

/// utility function used to place overlay elements quickly through user input without recompiling
fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin().read_line(&mut buffer).expect("Failed");
    buffer.trim().parse::<T>().unwrap_or_default()
}

/// The first screen, shown utpo 150 seconds before the next game, has no animations, so animation_counter is omitted
pub fn next_game(textures: &Textures, state: &State) {
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
}

/// Second screen, displayed between 150 and 30 seconds before the next game.
/// Animation counter holds state that represents animation progression. It must be an arbitrary float initlised to 0 and must live across function invocations.
pub fn roster(textures: &Textures, state: &State, animation_counter: &mut f32) {
    let offset = if state.snapshot.secs_in_period == 150 {
        *animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
        (0f32, -650f32).interpolate_linear(*animation_counter)
    } else {
        *animation_counter = 0f32;
        (0f32, -650f32).interpolate_linear(1f32)
    };
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_black_graphic(), 1090f32, 220f32, WHITE);
    draw_texture(*textures.team_white_graphic(), 150f32, 220f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, offset, WHITE);
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
        805f32 + offset,
        TextParams {
            font: textures.font(),
            font_size: 50,
            ..Default::default()
        },
    );
    draw_text_ex(
        state.black.to_uppercase().as_str(),
        1240f32,
        805f32 + offset,
        TextParams {
            font: textures.font(),
            font_size: 45,
            ..Default::default()
        },
    );
}

/// Display final scores after game is done
pub fn final_scores(textures: &Textures) {
    draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.final_score_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.team_information_graphic(), 0_f32, 0f32, WHITE);
}

/// displayed from 30 seconds before a game begins.
pub fn pre_game_display(textures: &Textures, state: &State, animation_counter: &mut f32) {
    if state.snapshot.secs_in_period > 15 {
        draw_texture(*textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(*textures.bottom_graphic(), 0_f32, 0f32, WHITE);
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
    } else if state.snapshot.secs_in_period == 15 {
        // animate a fade on the fifteenth second
        *animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period

        let offset = (255f32, 0f32).interpolate_linear(*animation_counter) as u8;
        draw_texture(
            *textures.atlantis_logo_graphic(),
            0_f32,
            0f32,
            Color::from_rgba(255, 255, 255, offset),
        );
        draw_texture(
            *textures.bottom_graphic(),
            0_f32,
            0f32,
            Color::from_rgba(255, 255, 255, offset),
        );
    } else {
        *animation_counter = 0f32;
    }
    draw_texture(*textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.time_and_game_state_graphic(), 0_f32, 0f32, WHITE);
}

/// Display info during game play
pub fn in_game_display(textures: &Textures, state: &State, animation_counter: &mut f32) {
    //animate the state and time graphic to the left at 895 secs (5 seconds since period started)
    let offset = if state.snapshot.secs_in_period == 895 {
        *animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
        (0f32, -200f32).interpolate_linear(*animation_counter)
    } else if state.snapshot.secs_in_period > 895 {
        (0f32, -200f32).interpolate_linear(0f32)
    } else {
        *animation_counter = 0f32;
        (0f32, -200f32).interpolate_linear(1f32)
    };
    draw_texture(*textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.in_game_mask(), 200_f32 + offset, 0f32, WHITE);
    draw_texture(*textures.time_and_game_state_graphic(), offset, 0f32, WHITE);
}

pub fn half_time_display(textures: &Textures) {
    draw_texture(*textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
    draw_texture(*textures.time_and_game_state_graphic(), 0f32, 0f32, WHITE);
}
