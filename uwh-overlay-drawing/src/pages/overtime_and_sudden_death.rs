use super::center_text_offset;
use super::PageRenderer;
use crate::State;
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;

impl PageRenderer {
    /// Display during overtime. Has no animations
    pub fn overtime_and_sudden_death_display(&mut self, state: &State) {
        draw_texture(self.textures.team_bar_graphic, 0_f32, 0f32, WHITE);
        if self.is_alpha_mode {
            draw_texture(self.textures.in_game_mask, 0f32, 0f32, WHITE);
            draw_texture(
                self.textures.time_and_game_state_graphic,
                -200f32,
                0f32,
                WHITE,
            );
            draw_rectangle(79f32, 39f32, 70f32, 33f32, WHITE);
            draw_rectangle(79f32, 75f32, 70f32, 33f32, WHITE);
        } else {
            draw_texture(
                self.textures.time_and_game_state_graphic,
                -200f32,
                0f32,
                WHITE,
            );
            let min = state.snapshot.secs_in_period / 60;
            let secs = state.snapshot.secs_in_period % 60;
            let x_off = center_text_offset!(
                90f32,
                format!("{}:{}", min, secs).as_str(),
                50,
                self.textures.font
            );
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                230f32 + x_off,
                95f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 50,
                    color: if [GamePeriod::SuddenDeath, GamePeriod::PreSuddenDeath]
                        .contains(&state.snapshot.current_period)
                    {
                        Color::from_rgba(255, 150, 0, 255)
                    } else {
                        Color::from_rgba(255, 0, 0, 255)
                    },
                    ..Default::default()
                },
            );
            let ot_text = match state.snapshot.current_period {
                GamePeriod::OvertimeFirstHalf => "OVERTIME 1ST HALF",
                GamePeriod::OvertimeSecondHalf => "OVERTIME 2ND HALF",
                GamePeriod::OvertimeHalfTime => "OVERTIME HALF TIME",
                GamePeriod::SuddenDeath => "SUDDEN DEATH",
                GamePeriod::PreSuddenDeath => "PRE SUDDEN DEATH",
                _ => "PRE OVERTIME",
            };
            let x_off = center_text_offset!(100f32, ot_text, 20, self.textures.font);
            draw_text_ex(
                ot_text,
                220f32 + x_off,
                45f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 20,
                    color: if [GamePeriod::SuddenDeath, GamePeriod::PreSuddenDeath]
                        .contains(&state.snapshot.current_period)
                    {
                        Color::from_rgba(255, 150, 0, 255)
                    } else {
                        Color::from_rgba(255, 0, 0, 255)
                    },
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
        }
        draw_text_ex(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: self.textures.font,
                font_size: 30,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: self.textures.font,
                font_size: 30,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
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
