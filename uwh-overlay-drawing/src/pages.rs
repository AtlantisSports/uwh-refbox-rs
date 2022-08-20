use crate::{load_images::Textures, State};
use macroquad::prelude::*;

trait Interpolate {
    /// `value` must be a floater varying from 0 to 1, denoting the lowest to highest limits of the range
    fn interpolate_linear(&self, value: f32) -> f32;
}

impl Interpolate for (f32, f32) {
    fn interpolate_linear(&self, value: f32) -> f32 {
        (self.1 - self.0).mul_add(value, self.0)
    }
}

#[allow(dead_code)]
/// Utility function used to place overlay elements quickly through user input without recompiling
fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to init stdin");
    buffer.trim().parse::<T>().unwrap_or_default()
}

pub struct PageRenderer {
    pub animation_counter: f32,
    pub is_alpha_mode: bool,
    pub textures: Textures,
}

impl PageRenderer {
    /// The Next Game screen, shown up to 150 seconds before the next game
    pub fn next_game(&mut self, state: &State) {
        draw_texture(*self.textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(*self.textures.bottom_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(
            *self.textures.team_information_graphic(),
            0_f32,
            0f32,
            WHITE,
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            1345f32,
            805f32,
            TextParams {
                font: self.textures.font(),
                font_size: 45,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            200f32,
            805f32,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
        if !self.is_alpha_mode {
            if let Some(flag) = state.w_flag {
                draw_texture_ex(
                    flag,
                    580f32,
                    738f32,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(180f32, 100f32)),
                        ..Default::default()
                    },
                );
            }
            if let Some(flag) = state.b_flag {
                draw_texture_ex(
                    flag,
                    1163f32,
                    738f32,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(180f32, 100f32)),
                        ..Default::default()
                    },
                );
            }
            let min = state.snapshot.secs_in_period / 60;
            let secs = state.snapshot.secs_in_period % 60;
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                923f32,
                1020f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 50,
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                905f32,
                1044f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 20,
                    ..Default::default()
                },
            );
        }
    }

    /// Roster screen, displayed between 150 and 30 seconds before the next game.
    pub fn roster(&mut self, state: &State) {
        let offset = if state.snapshot.secs_in_period == 150 {
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
            (0f32, -650f32).interpolate_linear(self.animation_counter)
        } else {
            self.animation_counter = 0f32;
            (0f32, -650f32).interpolate_linear(1f32)
        };
        draw_texture(*self.textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(*self.textures.bottom_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(*self.textures.team_black_graphic(), 1090f32, 220f32, WHITE);
        draw_texture(*self.textures.team_white_graphic(), 150f32, 220f32, WHITE);
        draw_texture(
            *self.textures.team_information_graphic(),
            0_f32,
            offset,
            WHITE,
        );
        draw_texture(
            *self.textures.team_white_graphic(),
            150f32,
            220f32 + 60f32,
            WHITE,
        );
        draw_texture(
            *self.textures.team_black_graphic(),
            1090f32,
            220f32 + 60f32,
            WHITE,
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            1345f32,
            805f32 + offset,
            TextParams {
                font: self.textures.font(),
                font_size: 45,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            200f32,
            805f32 + offset,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
        if !self.is_alpha_mode {
            if let Some(flag) = state.w_flag {
                draw_texture_ex(
                    flag,
                    580f32,
                    738f32 + offset,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(180f32, 100f32)),
                        ..Default::default()
                    },
                );
            }
            if let Some(flag) = state.b_flag {
                draw_texture_ex(
                    flag,
                    1163f32,
                    738f32 + offset,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(180f32, 100f32)),
                        ..Default::default()
                    },
                );
            }
            let min = state.snapshot.secs_in_period / 60;
            let secs = state.snapshot.secs_in_period % 60;
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                923f32,
                1020f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 50,
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                905f32,
                1044f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 20,
                    ..Default::default()
                },
            );
        }
    }

    /// Display final scores after game is done
    pub fn final_scores(&mut self, state: &State) {
        draw_texture(*self.textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(*self.textures.final_score_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(
            *self.textures.team_information_graphic(),
            0_f32,
            0f32,
            WHITE,
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            340f32,
            805f32,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            1240f32,
            805f32,
            TextParams {
                font: self.textures.font(),
                font_size: 45,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.snapshot.b_score.to_string().as_str(),
            1400f32,
            580f32,
            TextParams {
                font: self.textures.font(),
                font_size: 180,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.snapshot.w_score.to_string().as_str(),
            430f32,
            580f32,
            TextParams {
                font: self.textures.font(),
                font_size: 180,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
    }

    /// Displayed from 30 seconds before a game begins.
    pub fn pre_game_display(&mut self, state: &State) {
        if state.snapshot.secs_in_period > 15 {
            draw_texture(*self.textures.atlantis_logo_graphic(), 0_f32, 0f32, WHITE);
            draw_texture(*self.textures.bottom_graphic(), 0_f32, 0f32, WHITE);
            let min = state.snapshot.secs_in_period / 60;
            let secs = state.snapshot.secs_in_period % 60;
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                923f32,
                1020f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 50,
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                905f32,
                1044f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 20,
                    ..Default::default()
                },
            );
        } else if state.snapshot.secs_in_period == 15 && self.is_alpha_mode {
            // animate a fade on the fifteenth second in the alpha stream
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period

            let offset = (255f32, 0f32).interpolate_linear(self.animation_counter) as u8;
            draw_texture(
                *self.textures.atlantis_logo_graphic(),
                0_f32,
                0f32,
                Color::from_rgba(255, 255, 255, offset),
            );
            draw_texture(
                *self.textures.bottom_graphic(),
                0_f32,
                0f32,
                Color::from_rgba(255, 255, 255, offset),
            );
        } else {
            self.animation_counter = 0f32;
        }
        draw_texture(*self.textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(
            *self.textures.time_and_game_state_graphic(),
            0_f32,
            0f32,
            WHITE,
        );
    }

    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        //animate the state and time graphic to the left at 895 secs (5 seconds since period started)
        let offset = if state.snapshot.secs_in_period == 895 {
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
            (0f32, -200f32).interpolate_linear(self.animation_counter)
        } else if state.snapshot.secs_in_period > 895 {
            (0f32, -200f32).interpolate_linear(0f32)
        } else {
            self.animation_counter = 0f32;
            (0f32, -200f32).interpolate_linear(1f32)
        };
        draw_texture(*self.textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(*self.textures.in_game_mask(), 200_f32 + offset, 0f32, WHITE);
        draw_texture(
            *self.textures.time_and_game_state_graphic(),
            offset,
            0f32,
            WHITE,
        );
    }

    /// Display during overtime. Has no animations
    pub fn overtime_display(&mut self) {
        draw_texture(*self.textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
        draw_texture(
            *self.textures.time_and_game_state_graphic(),
            0f32,
            0f32,
            WHITE,
        );
    }

    // Shown every time a goal is made for five seconds. A second each for fade in and out.
    // Must use a secondary animation counter because this is called along with other draw functions
    // pub fn show_goal_graphic(&mut self) {
    //     //animate fade for the first second
    //     let offset = if self.animation_counter < 1f32 {
    //         self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
    //         (0f32, 255f32).interpolate_linear(self.animation_counter)
    //     } else if self.animation_counter < 4f32 {
    //         self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period

    //         (0f32, 255f32).interpolate_linear(1f32)
    //     } else if self.animation_counter < 5f32 {
    //         //animate fade out in the last one second
    //         self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
    //         (0f32, 255f32).interpolate_linear(5f32 - self.animation_counter)
    //     } else {
    //         self.animation_counter = 0f32;
    //         0f32
    //     } as u8;
    //     draw_texture(
    //         *self.textures.team_white_graphic(),
    //         25f32,
    //         150f32,
    //         Color::from_rgba(255, 255, 255, offset),
    //     );
    // }
}
