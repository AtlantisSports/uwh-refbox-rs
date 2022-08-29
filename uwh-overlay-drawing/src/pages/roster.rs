use super::center_text_offset;
use super::Interpolate;
use super::PageRenderer;
use crate::State;
use macroquad::prelude::*;

impl PageRenderer {
    /// Roster screen, displayed between 150 and 30 seconds before the next game.
    pub fn roster(&mut self, state: &State) {
        let offset = if state.snapshot.secs_in_period == 150 {
            self.animation_counter += 1f32 / 60f32; // inverse of number of frames in transition period
            (0f32, -650f32).interpolate_linear(self.animation_counter)
        } else {
            self.animation_counter = 0f32;
            (0f32, -650f32).interpolate_linear(1f32)
        };
        draw_texture(self.textures.atlantis_logo_graphic, 0_f32, 0f32, WHITE);
        draw_texture(self.textures.bottom_graphic, 0_f32, 0f32, WHITE);
        draw_texture(self.textures.team_black_graphic, 1090f32, 220f32, WHITE);
        draw_texture(self.textures.team_white_graphic, 150f32, 220f32, WHITE);
        draw_texture(self.textures.team_information_graphic, 0_f32, offset, WHITE);
        draw_texture(
            self.textures.team_white_graphic,
            150f32,
            220f32 + 60f32,
            WHITE,
        );
        draw_texture(
            self.textures.team_black_graphic,
            1090f32,
            220f32 + 60f32,
            WHITE,
        );
        let x_off = center_text_offset!(
            200f32,
            state.black.team_name.to_uppercase().as_str(),
            45,
            self.textures.font
        );
        draw_text_ex(
            state.black.team_name.to_uppercase().as_str(),
            1350f32 + x_off,
            805f32 + offset,
            TextParams {
                font: self.textures.font,
                font_size: 45,
                ..Default::default()
            },
        );
        let x_off = center_text_offset!(
            200f32,
            state.black.team_name.to_uppercase().as_str(),
            45,
            self.textures.font
        );
        draw_text_ex(
            state.white.team_name.to_uppercase().as_str(),
            120f32 + x_off,
            805f32 + offset,
            TextParams {
                font: self.textures.font,
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
                    self.textures.font.into(),
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
        }
    }
}
