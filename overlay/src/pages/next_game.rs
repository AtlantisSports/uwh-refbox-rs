use super::center_text_offset;
use super::draw_texture_both;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
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

        let (x_off, text) = center_text_offset!(
            217f32,
            state.black.team_name.to_uppercase().as_str(),
            45,
            self.assets.font
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
        let (x_off, text) = center_text_offset!(
            220f32,
            state.white.team_name.to_uppercase().as_str(),
            45,
            self.assets.font
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
        if state.white.flag.is_some() {
            draw_rectangle(2500f32, 738f32, 180f32, 100f32, WHITE);
        }
        if state.black.flag.is_some() {
            draw_rectangle(3083f32, 738f32, 180f32, 100f32, WHITE);
        }
        if let Some(flag) = state.white.flag {
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
        if let Some(flag) = state.black.flag {
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
            self.assets.font
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
        let (x_off, text) =
            center_text_offset!(124f32, state.start_time.as_str(), 25, self.assets.font);
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
        let (x_off, text) = center_text_offset!(110f32, &state.pool, 25, self.assets.font);
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
                format!("0{}", min)
            } else {
                format!("{}", min)
            },
            if secs < 10 {
                format!("0{}", secs)
            } else {
                format!("{}", secs)
            }
        );
        let (x_off, text) = center_text_offset!(90f32, text.as_str(), 50, self.assets.font);
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
