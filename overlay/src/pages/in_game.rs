use super::draw_texture_both;
use super::fit_text;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::pages::draw_texture_both_ex;
use crate::pages::Justify;
use crate::State;
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
use uwh_common::game_snapshot::TimeoutSnapshot;

impl PageRenderer {
    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        let mut time = Instant::now()
            .duration_since(self.animation_register2)
            .as_f64() as f32;
        let (timeout_offset, timeout_alpha_offset) =
            if state.snapshot.timeout != TimeoutSnapshot::None {
                if self.last_snapshot_timeout == TimeoutSnapshot::None {
                    // if this is a new timeout period
                    self.animation_register2 = Instant::now();
                    time = 0.0f32;
                }
                self.last_snapshot_timeout = state.snapshot.timeout;
                if time < 1f32 {
                    (
                        (0f32, -200f32).interpolate_linear(1f32 - time),
                        (0f32, 1f32).interpolate_exponential_end(time),
                    )
                } else {
                    (
                        (0f32, -200f32).interpolate_linear(0f32),
                        (0f32, 1f32).interpolate_exponential_end(1f32),
                    )
                }
            } else if self.last_snapshot_timeout != TimeoutSnapshot::None {
                // if a timeout period just finished, and fade out is just starting
                if !self.animation_register3 {
                    self.animation_register3 = true;
                    self.animation_register2 = Instant::now();
                    time = 0.0f32;
                }
                // when fade out is done
                if time > 1f32 {
                    self.animation_register3 = false;
                    self.animation_register2 = Instant::now();
                    self.last_snapshot_timeout = TimeoutSnapshot::None;
                    (
                        (0f32, -200f32).interpolate_linear(1f32),
                        (0f32, 1f32).interpolate_exponential_end(0f32),
                    )
                } else {
                    (
                        (0f32, -200f32).interpolate_linear(time),
                        (0f32, 1f32).interpolate_exponential_end(1f32 - time),
                    )
                }
            } else {
                // return any values when both are None, cause we won't be redering anyways
                (
                    (0f32, -200f32).interpolate_linear(0f32),
                    (0f32, 1f32).interpolate_exponential_end(1f32),
                )
            };
        match self.last_snapshot_timeout {
            // draw text for each type of penalty
            TimeoutSnapshot::Ref(_) => {
                draw_texture_both!(
                    self.assets.referee_timout,
                    timeout_offset + 580f32,
                    35f32,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    }
                );
                draw_text_both_ex!(
                    "REFEREE",
                    675f32 + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
                draw_text_both_ex!(
                    "TIMEOUT",
                    680f32 + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::White(time) => {
                draw_texture_both!(
                    self.assets.white_timout,
                    timeout_offset + 580f32,
                    35f32,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    }
                );
                draw_text_both_ex!(
                    "WHITE",
                    675f32 + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
                draw_text_both_ex!(
                    "TIMEOUT",
                    665f32 + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
                draw_text_both_ex!(
                    format!("{time}").as_str(),
                    773f32 + timeout_offset,
                    90f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 50,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 50,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::Black(time) => {
                draw_texture_both!(
                    self.assets.black_timout,
                    timeout_offset + 580f32,
                    35f32,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    }
                );
                draw_text_both!(
                    "BLACK",
                    675f32 + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
                draw_text_both!(
                    "TIMEOUT",
                    665f32 + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
                draw_text_both!(
                    format!("{time}").as_str(),
                    773f32 + timeout_offset,
                    90f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 50,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::PenaltyShot(_) => {
                draw_texture_both!(
                    self.assets.penalty,
                    timeout_offset + 580f32,
                    35f32,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    }
                );
                draw_text_both_ex!(
                    "PENALTY",
                    675f32 + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
                draw_text_both_ex!(
                    "SHOT",
                    690f32 + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 20,
                        color: Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        },
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::None => {} // this is ugly. `TimeoutSnapshot` must be made an `Option`
        }

        draw_texture_both!(self.assets.team_bar, 26f32, 37f32, WHITE);
        draw_text_both_ex!(
            &state.white.team_name,
            if state.white.flag.is_some() {
                160f32
            } else {
                79f32
            },
            64f32,
            TextParams {
                font: self.assets.font,
                font_size: 20,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.assets.font,
                font_size: 20,
                color: WHITE,
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
                font: self.assets.font,
                font_size: 20,
                color: WHITE,
                ..Default::default()
            }
        );
        draw_texture_both!(self.assets.time_and_game_state, 367f32, 18f32, WHITE);
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
            430f32 + x_off,
            67f32,
            TextParams {
                font: self.assets.font,
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
            478f32,
            100f32,
            TextParams {
                font: self.assets.font,
                font_size: 20,
                ..Default::default()
            },
        );
        if let Some(flag) = &state.white.flag {
            draw_texture_both_ex!(
                flag,
                79f32,
                39f32,
                WHITE,
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
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(70f32, 33f32)),
                    ..Default::default()
                }
            );
        }

        draw_text_both!(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: self.assets.font,
                font_size: 30,
                ..Default::default()
            }
        );
        draw_text_both_ex!(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: self.assets.font,
                font_size: 30,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.assets.font,
                font_size: 30,
                color: WHITE,
                ..Default::default()
            }
        );

        if let Some(logo) = self.assets.tournament_logo.as_ref() {
            let x = 1900f32 - logo.color.width();
            draw_texture_both!(logo, x, 20f32, WHITE);
        }
    }
}
