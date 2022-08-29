use super::center_text_offset;
use super::Interpolate;
use super::PageRenderer;
use crate::State;
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
use uwh_common::game_snapshot::TimeoutSnapshot;

impl PageRenderer {
    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        // animate the state and time graphic to the left at 895 secs (5 seconds since period started)
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
            if state.snapshot.timeout != TimeoutSnapshot::None {
                self.secondary_animation_counter += 1f32 / 60f32;
            }
            (
                (0f32, -200f32).interpolate_linear(1f32),
                (255f32, 0f32).interpolate_linear(1f32) as u8,
            )
        };
        draw_texture(self.textures.team_bar_graphic, 0_f32, 0f32, WHITE);
        if self.is_alpha_mode {
            draw_texture(
                self.textures.in_game_mask,
                200_f32 + position_offset,
                0f32,
                WHITE,
            );
            draw_texture(
                self.textures.time_and_game_state_graphic,
                position_offset,
                0f32,
                WHITE,
            );
            draw_rectangle(79f32, 39f32, 70f32, 33f32, WHITE);
            draw_rectangle(79f32, 75f32, 70f32, 33f32, WHITE);
        } else {
            // if state.snapshot.timeout != TimeoutSnapshot::None {
            // draw_texture(
            //     *self.textures,
            //     position_offset,
            //     0f32,
            //     WHITE,
            // );
            // }
            draw_text_ex(
                state.white.team_name.to_uppercase().as_str(),
                160f32,
                64f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 20,
                    color: Color::from_rgba(0, 0, 0, alpha_offset),
                    ..Default::default()
                },
            );
            draw_text_ex(
                state.black.team_name.to_uppercase().as_str(),
                160f32,
                100f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 20,
                    color: Color::from_rgba(255, 255, 255, alpha_offset),
                    ..Default::default()
                },
            );
            draw_texture(
                self.textures.time_and_game_state_graphic,
                position_offset,
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
                430f32 + position_offset + x_off,
                67f32,
                TextParams {
                    font: self.textures.font,
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
                    font: self.textures.font,
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
}
