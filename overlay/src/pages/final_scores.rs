use super::center_text_offset;
use super::draw_texture_both;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::State;
use macroquad::prelude::*;

impl PageRenderer {
    /// Display final scores after game is done
    pub fn final_scores(&mut self, state: &State) {
        draw_texture_both!(self.textures.atlantis_logo_graphic, 836f32, 725f32, WHITE);
        draw_texture_both!(self.textures.final_score_graphic, 314f32, 347f32, WHITE);
        draw_texture_both!(
            self.textures.team_information_graphic,
            130f32,
            710f32,
            WHITE
        );
        let (x_off, text) = center_text_offset!(
            217f32,
            state.black.team_name.to_uppercase().as_str(),
            45,
            self.textures.font
        );
        draw_text_both!(
            text.as_str(),
            1350f32 + x_off,
            805f32,
            TextParams {
                font: self.textures.font,
                font_size: 45,
                ..Default::default()
            }
        );
        let (x_off, text) = center_text_offset!(
            220f32,
            state.white.team_name.to_uppercase().as_str(),
            45,
            self.textures.font
        );
        draw_text_both_ex!(
            text.as_str(),
            135f32 + x_off,
            805f32,
            TextParams {
                font: self.textures.font,
                font_size: 45,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.textures.font,
                font_size: 45,
                color: WHITE,
                ..Default::default()
            }
        );
        if state.white_flag.is_some() {
            draw_rectangle(2500f32, 738f32, 180f32, 100f32, WHITE);
        }
        if state.black_flag.is_some() {
            draw_rectangle(3083f32, 738f32, 180f32, 100f32, WHITE);
        }
        if let Some(flag) = state.white_flag {
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
        if let Some(flag) = state.black_flag {
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
        let (x_off, text) = center_text_offset!(
            135f32,
            format!("GAME #{}", &state.game_id.to_string()).as_str(),
            25,
            self.textures.font
        );
        draw_text_ex(
            text.as_str(),
            830f32 + x_off,
            745f32,
            TextParams {
                font: self.textures.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) =
            center_text_offset!(124f32, state.start_time.as_str(), 25, self.textures.font);
        draw_text_ex(
            text.as_str(),
            838f32 + x_off,
            780f32,
            TextParams {
                font: self.textures.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = center_text_offset!(110f32, &state.pool, 25, self.textures.font);
        draw_text_ex(
            text.as_str(),
            855f32 + x_off,
            815f32,
            TextParams {
                font: self.textures.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = center_text_offset!(
            145f32,
            state.snapshot.b_score.to_string().as_str(),
            180,
            self.textures.font
        );
        draw_text_both!(
            text.as_str(),
            1295f32 + x_off,
            580f32,
            TextParams {
                font: self.textures.font,
                font_size: 180,
                ..Default::default()
            }
        );
        let (x_off, text) = center_text_offset!(
            145f32,
            state.snapshot.w_score.to_string().as_str(),
            180,
            self.textures.font
        );
        draw_text_both_ex!(
            text.as_str(),
            340f32 + x_off,
            580f32,
            TextParams {
                font: self.textures.font,
                font_size: 180,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.textures.font,
                font_size: 180,
                color: WHITE,
                ..Default::default()
            }
        );
    }
}
