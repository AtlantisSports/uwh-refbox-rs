use super::draw_texture_both;
use super::fit_text;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::pages::draw_texture_both_ex;
use crate::{pages::Justify, State};
use coarsetime::Instant;
use macroquad::prelude::*;

impl PageRenderer {
    /// Displayed from 30 seconds before a game begins.
    pub fn pre_game_display(&mut self, state: &State) {
        let (sponsor_alpha, midfade_alpha, tandg_alpha) = match state.snapshot.secs_in_period {
            31.. => {
                self.animation_register1 = Instant::now();
                (1f32, 1f32, 1f32)
            }
            30 => (
                (1f32, 0f32).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register1)
                        .as_f64() as f32,
                ),
                (0f32, 1f32).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register1)
                        .as_f64() as f32,
                ),
                1f32,
            ),
            16.. => {
                self.animation_register1 = Instant::now();
                (0f32, 1f32, 1f32)
            }
            15 => {
                (
                    0f32, // animate a fade on the fifteenth second
                    (1f32, 0f32).interpolate_linear(
                        Instant::now()
                            .duration_since(self.animation_register1)
                            .as_f64() as f32,
                    ),
                    1f32,
                )
            }
            _ => {
                self.animation_register1 = Instant::now();
                (0f32, 0f32, 1f32)
            }
        };

        if let Some(sponsor_logo) = &state.sponsor_logo {
            let x = (1920f32 - sponsor_logo.color.width()) / 2f32;
            if sponsor_alpha > 0f32 {
                draw_texture_both!(
                    sponsor_logo,
                    x,
                    200f32,
                    Color {
                        a: sponsor_alpha,
                        ..WHITE
                    }
                );
            }
        }

        if midfade_alpha > 0f32 {
            // draw_texture_both!(
            //     self.assets.atlantis_logo,
            //     836f32,
            //     725f32,
            //     Color {
            //         a: midfade_alpha,
            //         ..WHITE
            //     }
            // );
            draw_texture_both!(
                self.assets.bottom,
                822f32,
                977f32,
                Color {
                    a: midfade_alpha,
                    ..WHITE
                }
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
            let (x_off, text) = fit_text(180f32, &text, 50, &self.assets.font, Justify::Center);
            draw_text_ex(
                text.as_str(),
                870f32 + x_off,
                1020f32,
                TextParams {
                    font: Some(&self.assets.font),
                    font_size: 50,
                    color: Color {
                        a: midfade_alpha,
                        ..WHITE
                    },
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                905f32,
                1044f32,
                TextParams {
                    font: Some(&self.assets.font),
                    font_size: 20,
                    color: Color {
                        a: midfade_alpha,
                        ..WHITE
                    },

                    ..Default::default()
                },
            );
        }

        draw_texture_both!(
            self.assets.team_bar,
            26f32,
            37f32,
            Color {
                a: tandg_alpha,
                ..WHITE
            }
        );
        draw_text_both!(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 30,
                color: Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                ..Default::default()
            }
        );
        draw_text_both_ex!(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 30,
                color: Color {
                    a: tandg_alpha,
                    ..BLACK
                },
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
                font_size: 30,
                color: Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                ..Default::default()
            }
        );

        draw_text_both_ex!(
            &state.white.team_name,
            if state.white.flag.is_some() {
                160f32
            } else {
                79f32
            },
            64f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 20,
                color: Color {
                    a: tandg_alpha,
                    ..BLACK
                },
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
                font_size: 20,
                color: Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                ..Default::default()
            }
        );
        draw_text_both!(
            &state.black.team_name,
            if state.black.flag.is_some() {
                160f32
            } else {
                79f32
            },
            100f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 20,
                color: Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                ..Default::default()
            }
        );
        draw_texture_both!(
            self.assets.time_and_game_state,
            367f32,
            18f32,
            Color {
                a: tandg_alpha,
                ..WHITE
            }
        );
        let min = state.half_play_duration.unwrap_or(900) / 60;
        let secs = state.half_play_duration.unwrap_or(900) % 60;
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
        let (x_off, text) = fit_text(180f32, &text, 50, &self.assets.font, Justify::Center);
        draw_text_ex(
            text.as_str(),
            430f32 + x_off,
            67f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 50,
                color: Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                ..Default::default()
            },
        );
        draw_text_ex(
            "1ST HALF",
            478f32,
            100f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 20,
                color: Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                ..Default::default()
            },
        );
        if let Some(flag) = &state.white.flag {
            draw_texture_both_ex!(
                flag,
                79f32,
                39f32,
                Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                }
            );
        }
        if let Some(flag) = &state.black.flag {
            draw_texture_both_ex!(
                flag,
                79f32,
                75f32,
                Color {
                    a: tandg_alpha,
                    ..WHITE
                },
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                }
            );
        }

        if let Some(logo) = state.tournament_logo.as_ref() {
            let x = 1900f32 - logo.color.width();
            draw_texture_both!(logo, x, 20f32, WHITE);
        }
    }
}
