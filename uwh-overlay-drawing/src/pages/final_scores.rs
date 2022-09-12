use super::center_text_offset;
use super::PageRenderer;
use crate::State;
use macroquad::prelude::*;

impl PageRenderer {
    /// Display final scores after game is done
    pub fn final_scores(&mut self, state: &State) {
        draw_texture(self.textures.atlantis_logo_graphic, 0_f32, 0f32, WHITE);
        draw_texture(self.textures.final_score_graphic, 0_f32, 0f32, WHITE);
        draw_texture(self.textures.team_information_graphic, 0_f32, 0f32, WHITE);
        draw_text_ex(
            state.white.team_name.to_uppercase().as_str(),
            340f32,
            805f32,
            TextParams {
                font: self.textures.font,
                font_size: 50,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.black.team_name.to_uppercase().as_str(),
            1240f32,
            805f32,
            TextParams {
                font: self.textures.font,
                font_size: 45,
                ..Default::default()
            },
        );
        if !self.is_alpha_mode {
            let x_off = center_text_offset!(
                135f32,
                format!("GAME ID: {}", &state.game_id.to_string()).as_str(),
                25,
                self.textures.font
            );
            draw_text_ex(
                format!("GAME ID: {}", &state.game_id.to_string()).as_str(),
                830f32 + x_off,
                745f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 25,
                    ..Default::default()
                },
            );
            let x_off = center_text_offset!(
                124f32,
                format!("{}", &state.start_time.to_string()).as_str(),
                25,
                self.textures.font
            );
            draw_text_ex(
                format!("{}", &state.start_time.to_string()).as_str(),
                838f32 + x_off,
                780f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 25,
                    ..Default::default()
                },
            );
            let x_off = center_text_offset!(
                110f32,
                format!("POOL: {}", &state.pool.to_string()).as_str(),
                25,
                self.textures.font
            );
            draw_text_ex(
                format!("POOL: {}", &state.pool.to_string()).as_str(),
                855f32 + x_off,
                815f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 25,
                    ..Default::default()
                },
            );
        }
        let x_off = center_text_offset!(
            145f32,
            state.snapshot.b_score.to_string().as_str(),
            180,
            self.textures.font
        );
        draw_text_ex(
            state.snapshot.b_score.to_string().as_str(),
            1295f32 + x_off,
            580f32,
            TextParams {
                font: self.textures.font,
                font_size: 180,
                ..Default::default()
            },
        );
        let x_off = center_text_offset!(
            145f32,
            state.snapshot.w_score.to_string().as_str(),
            180,
            self.textures.font
        );
        draw_text_ex(
            state.snapshot.w_score.to_string().as_str(),
            340f32 + x_off,
            580f32,
            TextParams {
                font: self.textures.font,
                font_size: 180,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
    }
}
