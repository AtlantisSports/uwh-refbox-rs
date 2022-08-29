use super::center_text_offset;
use super::get_input;
use super::Interpolate;
use super::PageRenderer;
use crate::State;
use macroquad::prelude::*;

impl PageRenderer {
    /// Displayed from 30 seconds before a game begins.
    pub fn pre_game_display(&mut self, state: &State) {
        if state.snapshot.secs_in_period > 15 {
            draw_texture(self.textures.atlantis_logo_graphic, 0_f32, 0f32, WHITE);
            draw_texture(self.textures.bottom_graphic, 0_f32, 0f32, WHITE);
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
                870f32 + x_off,
                1020f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 50,
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                905f32,
                1044f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 20,
                    ..Default::default()
                },
            );
        } else if state.snapshot.secs_in_period == 15 {
            // animate a fade on the fifteenth second in the alpha stream
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
            let offset = (255f32, 0f32).interpolate_linear(self.animation_counter) as u8;

            draw_texture(
                self.textures.atlantis_logo_graphic,
                0_f32,
                0f32,
                Color::from_rgba(255, 255, 255, if self.is_alpha_mode { offset } else { 255 }),
            );
            draw_texture(
                self.textures.bottom_graphic,
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
                self.textures.font
            );
            draw_text_ex(
                format!("{}:{}", min, secs).as_str(),
                870f32 + x_off,
                1020f32,
                TextParams {
                    font: self.textures.font,
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
                    font: self.textures.font,
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
        draw_texture(self.textures.team_bar_graphic, 0_f32, 0f32, WHITE);
        draw_texture(
            self.textures.time_and_game_state_graphic,
            0_f32,
            0f32,
            WHITE,
        );
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
                color: BLACK,
                ..Default::default()
            },
        );

        draw_text_ex(
            state.white.team_name.to_uppercase().as_str(),
            if let Some(_) = state.white_flag {
                160f32
            } else {
                79f32
            },
            64f32,
            TextParams {
                font: self.textures.font,
                font_size: 20,
                color: BLACK,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.black.team_name.to_uppercase().as_str(),
            if let Some(_) = state.black_flag {
                160f32
            } else {
                79f32
            },
            100f32,
            TextParams {
                font: self.textures.font,
                font_size: 20,
                ..Default::default()
            },
        );
        if let Some(flag) = state.white_flag {
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
        if let Some(flag) = state.black_flag {
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
                font: self.textures.font,
                font_size: 50,
                ..Default::default()
            },
        );
        draw_text_ex(
            "1ST HALF",
            478f32,
            100f32,
            TextParams {
                font: self.textures.font,
                font_size: 20,
                ..Default::default()
            },
        );
    }
}
