use super::draw_texture_both;
use super::fit_text;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::pages::draw_texture_both_ex;
use crate::pages::Justify;
use crate::State;
use coarsetime::Instant;
use macroquad::prelude::*;

impl PageRenderer {
    /// The Next Game screen, shown up to 150 seconds before the next game
    pub fn next_game(&mut self, state: &State) {
        self.animation_register1 = Instant::now();
        draw_texture_both!(self.assets.atlantis_logo, 836f32, 725f32, WHITE);
        draw_texture_both!(self.assets.bottom, 822f32, 977f32, WHITE);
        draw_texture_both!(self.assets.team_information, 130f32, 710f32, WHITE);

        if let Some(logo) = &state.tournament_logo {
            let x = 1900f32 - logo.color.width();
            draw_texture_both!(logo, x, 20f32, WHITE);
        }

        if let Some(sponsor_logo) = &state.sponsor_logo {
            let x = (1920f32 - sponsor_logo.color.width()) / 2f32;
            draw_texture_both!(sponsor_logo, x, 200f32, WHITE);
        }

        let (x_off, text) = fit_text(
            434f32,
            &state.black.team_name,
            45,
            self.assets.font,
            Justify::Center,
        );
        draw_text_both!(
            text.as_str(),
            1350f32 + x_off,
            805f32,
            TextParams {
                font: self.assets.font,
                font_size: 45,
                ..Default::default()
            }
        );
        let (x_off, text) = fit_text(
            440f32,
            &state.white.team_name,
            45,
            self.assets.font,
            Justify::Center,
        );
        draw_text_both_ex!(
            text.as_str(),
            135f32 + x_off,
            805f32,
            TextParams {
                font: self.assets.font,
                font_size: 45,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.assets.font,
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
            self.assets.font,
            Justify::Center,
        );
        draw_text_ex(
            text.as_str(),
            830f32 + x_off,
            745f32,
            TextParams {
                font: self.assets.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = fit_text(
            248f32,
            &state.start_time,
            25,
            self.assets.font,
            Justify::Center,
        );
        draw_text_ex(
            text.as_str(),
            838f32 + x_off,
            780f32,
            TextParams {
                font: self.assets.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let (x_off, text) = fit_text(220f32, &state.pool, 25, self.assets.font, Justify::Center);
        draw_text_ex(
            text.as_str(),
            855f32 + x_off,
            815f32,
            TextParams {
                font: self.assets.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let min = state.snapshot.secs_in_period / 60;
        let secs = state.snapshot.secs_in_period % 60;
        let text = format!(
            "{}:{}",
            if min < 10 {
                format!("0{min}")
            } else {
                format!("{min}")
            },
            if secs < 10 {
                format!("0{secs}")
            } else {
                format!("{secs}")
            }
        );
        let (x_off, text) = fit_text(180f32, &text, 50, self.assets.font, Justify::Center);
        draw_text_ex(
            text.as_str(),
            870f32 + x_off,
            1020f32,
            TextParams {
                font: self.assets.font,
                font_size: 50,
                ..Default::default()
            },
        );
        draw_text_ex(
            "NEXT GAME",
            907f32,
            1044f32,
            TextParams {
                font: self.assets.font,
                font_size: 20,
                ..Default::default()
            },
        );
    }
}
