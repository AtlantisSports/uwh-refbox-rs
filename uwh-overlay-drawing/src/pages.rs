use crate::{load_images::Textures, State};
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;

trait Interpolate {
    /// `value` must be a floater varying from 0 to 1, denoting the lowest to highest limits of the range
    fn interpolate_linear(&self, value: f32) -> f32;
}

impl Interpolate for (f32, f32) {
    fn interpolate_linear(&self, value: f32) -> f32 {
        (self.1 - self.0).mul_add(value, self.0)
    }
}

macro_rules! center_text_offset {
    ($field_width: expr, $string: expr, $font_size: literal, $font: expr) => {
        $field_width - measure_text($string, Some($font), $font_size, 1.0).width / 2f32
    };
}

#[allow(dead_code)]
/// Utility function used to place overlay elements quickly through user input without recompiling
fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to init stdin");
    buffer.trim().parse().unwrap_or_default()
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

        let x_off = center_text_offset!(
            200f32,
            state.black.to_uppercase().as_str(),
            45,
            self.textures.font()
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            1350f32 + x_off,
            805f32,
            TextParams {
                font: self.textures.font(),
                font_size: 45,
                ..Default::default()
            },
        );
        let x_off = center_text_offset!(
            200f32,
            state.black.to_uppercase().as_str(),
            45,
            self.textures.font()
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            120f32 + x_off,
            805f32,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
        if self.is_alpha_mode {
            if state.w_flag.is_some() {
                draw_rectangle(580f32, 738f32, 180f32, 100f32, WHITE);
            }
            if state.b_flag.is_some() {
                draw_rectangle(1163f32, 738f32, 180f32, 100f32, WHITE);
            }
        } else {
            if let Some(flag) = state.w_flag {
                draw_texture_ex(
                    flag,
                    580f32,
                    738f32,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(flag.width() / (flag.height() / 100f32), 100f32)),
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
            let x_off = center_text_offset!(
                90f32,
                format!("{}:{}", min, secs).as_str(),
                50,
                self.textures.font()
            );
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                870f32 + x_off,
                1020f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 50,
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                907f32,
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
        let x_off = center_text_offset!(
            200f32,
            state.black.to_uppercase().as_str(),
            45,
            self.textures.font()
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            1350f32 + x_off,
            805f32 + offset,
            TextParams {
                font: self.textures.font(),
                font_size: 45,
                ..Default::default()
            },
        );
        let x_off = center_text_offset!(
            200f32,
            state.black.to_uppercase().as_str(),
            45,
            self.textures.font()
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            120f32 + x_off,
            805f32 + offset,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
        if self.is_alpha_mode {
            if state.w_flag.is_some() {
                draw_rectangle(580f32, 738f32 + offset, 180f32, 100f32, WHITE);
            }
            if state.b_flag.is_some() {
                draw_rectangle(1163f32, 738f32 + offset, 180f32, 100f32, WHITE);
            }
        } else {
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
            let x_off: f32 = 90f32
                - measure_text(
                    format!("{}:{}", min, secs).as_str(),
                    self.textures.font().into(),
                    50,
                    1.0,
                )
                .width
                    / 2f32;
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                870f32 + x_off,
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
            let x_off = center_text_offset!(
                90f32,
                format!("{}:{}", min, secs).as_str(),
                50,
                self.textures.font()
            );
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                870f32 + x_off,
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
        } else if state.snapshot.secs_in_period == 15 {
            // animate a fade on the fifteenth second in the alpha stream
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
            let offset = (255f32, 0f32).interpolate_linear(self.animation_counter) as u8;

            draw_texture(
                *self.textures.atlantis_logo_graphic(),
                0_f32,
                0f32,
                Color::from_rgba(255, 255, 255, if self.is_alpha_mode { offset } else { 255 }),
            );
            draw_texture(
                *self.textures.bottom_graphic(),
                0_f32,
                0f32,
                Color::from_rgba(255, 255, 255, if self.is_alpha_mode { offset } else { 255 }),
            );
            let min = state.snapshot.secs_in_period / 60;
            let secs = state.snapshot.secs_in_period % 60;
            let x_off = center_text_offset!(
                90f32,
                format!("{}:{}", min, secs).as_str(),
                50,
                self.textures.font()
            );
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                870f32 + x_off,
                1020f32,
                TextParams {
                    font: self.textures.font(),
                    font_size: 50,
                    color: Color::from_rgba(
                        255,
                        255,
                        255,
                        if self.is_alpha_mode { offset } else { 255 },
                    ),
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
                    color: Color::from_rgba(
                        255,
                        255,
                        255,
                        if self.is_alpha_mode { offset } else { 255 },
                    ),

                    ..Default::default()
                },
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
        draw_text_ex(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: self.textures.font(),
                font_size: 30,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: self.textures.font(),
                font_size: 30,
                color: BLACK,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            160f32,
            64f32,
            TextParams {
                font: self.textures.font(),
                font_size: 20,
                color: BLACK,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            160f32,
            100f32,
            TextParams {
                font: self.textures.font(),
                font_size: 20,
                ..Default::default()
            },
        );
        if let Some(flag) = state.w_flag {
            draw_texture_ex(
                flag,
                79f32,
                39f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                },
            );
        }
        if let Some(flag) = state.b_flag {
            draw_texture_ex(
                flag,
                79f32,
                75f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                },
            );
        }
        draw_text_ex(
            "15:00",
            460f32,
            67f32,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                ..Default::default()
            },
        );
        draw_text_ex(
            "1ST HALF",
            478f32,
            100f32,
            TextParams {
                font: self.textures.font(),
                font_size: 20,
                ..Default::default()
            },
        );
    }

    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        //animate the state and time graphic to the left at 895 secs (5 seconds since period started)
        let (position_offset, alpha_offset) = if state.snapshot.secs_in_period == 895 {
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
            (
                (0f32, -200f32).interpolate_linear(self.animation_counter),
                (255f32, 0f32).interpolate_linear(self.animation_counter) as u8,
            )
        } else if state.snapshot.secs_in_period > 895 {
            (
                (0f32, -200f32).interpolate_linear(0f32),
                (255f32, 0f32).interpolate_linear(0f32) as u8,
            )
        } else {
            self.animation_counter = 0f32;
            (
                (0f32, -200f32).interpolate_linear(1f32),
                (255f32, 0f32).interpolate_linear(1f32) as u8,
            )
        };
        draw_texture(*self.textures.team_bar_graphic(), 0_f32, 0f32, WHITE);
        if self.is_alpha_mode {
            draw_texture(
                *self.textures.in_game_mask(),
                200_f32 + position_offset,
                0f32,
                WHITE,
            );
        }
        draw_text_ex(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: self.textures.font(),
                font_size: 30,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: self.textures.font(),
                font_size: 30,
                color: BLACK,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.white.to_uppercase().as_str(),
            160f32,
            64f32,
            TextParams {
                font: self.textures.font(),
                font_size: 20,
                color: Color::from_rgba(0, 0, 0, alpha_offset),
                ..Default::default()
            },
        );
        draw_text_ex(
            state.black.to_uppercase().as_str(),
            160f32,
            100f32,
            TextParams {
                font: self.textures.font(),
                font_size: 20,
                color: Color::from_rgba(255, 255, 255, alpha_offset),
                ..Default::default()
            },
        );
        draw_texture(
            *self.textures.time_and_game_state_graphic(),
            position_offset,
            0f32,
            WHITE,
        );
        if let Some(flag) = state.w_flag {
            draw_texture_ex(
                flag,
                79f32,
                39f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                },
            );
        }
        if let Some(flag) = state.b_flag {
            draw_texture_ex(
                flag,
                79f32,
                75f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                },
            );
        }
        let min = state.snapshot.secs_in_period / 60;
        let secs = state.snapshot.secs_in_period % 60;
        let x_off = center_text_offset!(
            90f32,
            format!("{}:{}", min, secs).as_str(),
            50,
            self.textures.font()
        );
        draw_text_ex(
            format!("{}:{}", min, secs).as_str(),
            430f32 + position_offset + x_off,
            67f32,
            TextParams {
                font: self.textures.font(),
                font_size: 50,
                ..Default::default()
            },
        );
        draw_text_ex(
            match state.snapshot.current_period {
                GamePeriod::FirstHalf => "1ST HALF",
                GamePeriod::SecondHalf => "2ND HALF",
                _ => "HALF TIME",
            },
            478f32 + position_offset,
            100f32,
            TextParams {
                font: self.textures.font(),
                font_size: 20,
                ..Default::default()
            },
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
