use super::PageRenderer;
use super::draw_texture_both;
use super::fit_text;
use crate::State;
use crate::pages::Justify;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::pages::draw_texture_both_ex;
use coarsetime::Instant;
use macroquad::prelude::*;

impl PageRenderer {
    /// Display final scores after game is done
    pub fn final_scores(&mut self, state: &State) {
        self.animation_register1 = Instant::now();
        draw_texture_both!(self.assets.atlantis_logo, 836f32, 725f32, WHITE);
        draw_texture_both!(self.assets.final_score, 314f32, 347f32, WHITE);

        if let Some(logo) = state.tournament_logo.as_ref() {
            let x = (1920f32 - logo.color.width()) / 2f32;
            let y = 675f32 - logo.color.height();
            draw_texture_both!(logo, x, y, WHITE);
        }

        draw_texture_both!(self.assets.team_information, 130f32, 710f32, WHITE);
        let (x_off, text) = fit_text(
            434f32,
            &state.black.team_name,
            45,
            &self.assets.font,
            Justify::Center,
        );
        draw_text_both!(
            text.as_str(),
            1350f32 + x_off,
            805f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 45,
                ..Default::default()
            }
        );
        let (x_off, text) = fit_text(
            440f32,
            &state.white.team_name,
            45,
            &self.assets.font,
            Justify::Center,
        );
        draw_text_both_ex!(
            text.as_str(),
            135f32 + x_off,
            805f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 45,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
                font_size: 45,
                color: WHITE,
                ..Default::default()
            }
        );
        if let Some(flag) = &state.white.flag {
            draw_texture_both_ex!(
                flag,
                580f32,
                738f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(180f32, 100f32)),
                    ..Default::default()
                }
            );
        }
        if let Some(flag) = &state.black.flag {
            draw_texture_both_ex!(
                flag,
                1163f32,
                738f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(180f32, 100f32)),
                    ..Default::default()
                }
            );
        }
        let (x_off, text) = fit_text(
            270f32,
            &format!("GAME #{}", &state.game_id.to_string()),
            25,
            &self.assets.font,
            Justify::Center,
        );
        draw_text_ex(
            text.as_str(),
            830f32 + x_off,
            745f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = fit_text(
            248f32,
            &state.start_time,
            25,
            &self.assets.font,
            Justify::Center,
        );
        draw_text_ex(
            text.as_str(),
            838f32 + x_off,
            780f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = fit_text(220f32, &state.pool, 25, &self.assets.font, Justify::Center);
        draw_text_ex(
            text.as_str(),
            855f32 + x_off,
            815f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = fit_text(
            290f32,
            &state.snapshot.b_score.to_string(),
            180,
            &self.assets.font,
            Justify::Center,
        );
        draw_text_both!(
            text.as_str(),
            1295f32 + x_off,
            580f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 180,
                ..Default::default()
            }
        );
        let (x_off, text) = fit_text(
            290f32,
            &state.snapshot.w_score.to_string(),
            180,
            &self.assets.font,
            Justify::Center,
        );
        draw_text_both_ex!(
            text.as_str(),
            340f32 + x_off,
            580f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 180,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
                font_size: 180,
                color: WHITE,
                ..Default::default()
            }
        );
    }
}
