use super::center_text_offset;
use super::draw_texture_both;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::State;
use crate::ALPHA_MAX;
use crate::ALPHA_MIN;
use coarsetime::Instant;
use macroquad::prelude::*;

impl PageRenderer {
    /// Displayed from 30 seconds before a game begins.
    pub fn pre_game_display(&mut self, state: &State) {
        match state.snapshot.secs_in_period {
            16.. => {
                draw_texture_both!(self.textures.atlantis_logo_graphic, 823f32, 712f32, WHITE);
                draw_texture_both!(self.textures.bottom_graphic, 822f32, 977f32, WHITE);
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
                let x_off = center_text_offset!(90f32, text.as_str(), 50, self.textures.font);
                draw_text_ex(
                    text.as_str(),
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
                self.animation_register1 = Instant::now();
            }
            15 => {
                // animate a fade on the fifteenth second
                let offset = (ALPHA_MAX, ALPHA_MIN).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register1)
                        .as_f64() as f32,
                ) as u8;

                draw_texture_both!(
                    self.textures.atlantis_logo_graphic,
                    823f32,
                    712f32,
                    Color::from_rgba(255, 255, 255, offset)
                );
                draw_texture_both!(
                    self.textures.bottom_graphic,
                    822f32,
                    977f32,
                    Color::from_rgba(255, 255, 255, offset)
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
                let x_off = center_text_offset!(90f32, text.as_str(), 50, self.textures.font);
                draw_text_ex(
                    text.as_str(),
                    870f32 + x_off,
                    1020f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 50,
                        color: Color::from_rgba(255, 255, 255, offset),
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
                        color: Color::from_rgba(255, 255, 255, offset),

                        ..Default::default()
                    },
                );
            }
            _ => {
                self.animation_register1 = Instant::now();
            }
        }
        draw_texture_both!(self.textures.team_bar_graphic, 26f32, 37f32, WHITE);
        draw_texture_both!(
            self.textures.time_and_game_state_graphic,
            367f32,
            18f32,
            WHITE
        );
        draw_text_both!(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: self.textures.font,
                font_size: 30,
                ..Default::default()
            }
        );
        draw_text_both_ex!(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: self.textures.font,
                font_size: 30,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.textures.font,
                font_size: 30,
                ..Default::default()
            }
        );

        draw_text_both_ex!(
            state.white.team_name.to_uppercase().as_str(),
            if state.white_flag.is_some() {
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
            TextParams {
                font: self.textures.font,
                font_size: 20,
                ..Default::default()
            }
        );
        draw_text_both!(
            state.black.team_name.to_uppercase().as_str(),
            if state.black_flag.is_some() {
                160f32
            } else {
                79f32
            },
            100f32,
            TextParams {
                font: self.textures.font,
                font_size: 20,
                ..Default::default()
            }
        );
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
        if state.white_flag.is_some() {
            draw_rectangle(1999f32, 39f32, 70f32, 33f32, WHITE);
        }
        if state.black_flag.is_some() {
            draw_rectangle(1999f32, 75f32, 70f32, 33f32, WHITE);
        }
    }
}
