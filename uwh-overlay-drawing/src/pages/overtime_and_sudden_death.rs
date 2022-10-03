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
use uwh_common::game_snapshot::GamePeriod;
use uwh_common::game_snapshot::TimeoutSnapshot;

impl PageRenderer {
    /// Display during overtime. Has no animations
    pub fn overtime_and_sudden_death_display(&mut self, state: &State) {
        let mut time = Instant::now()
            .duration_since(self.animation_register2)
            .as_f64() as f32;
        // animate the state and time graphic to the left at 895 secs (5 seconds since period started)
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
                        (ALPHA_MIN, ALPHA_MAX).interpolate_exponential_end(time),
                    )
                } else {
                    (
                        (0f32, -200f32).interpolate_linear(0f32),
                        (ALPHA_MIN, ALPHA_MAX).interpolate_exponential_end(1f32),
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
                        (ALPHA_MIN, ALPHA_MAX).interpolate_exponential_end(0f32),
                    )
                } else {
                    (
                        (0f32, -200f32).interpolate_linear(time),
                        (ALPHA_MIN, ALPHA_MAX).interpolate_exponential_end(1f32 - time),
                    )
                }
            } else {
                // return any values when both are None, cause we won't be redering anyways
                (
                    (0f32, -200f32).interpolate_linear(0f32),
                    (ALPHA_MIN, ALPHA_MAX).interpolate_exponential_end(1f32),
                )
            };

        draw_texture_both!(self.textures.team_bar_graphic, 26f32, 37f32, WHITE);
        draw_texture_both!(self.textures.in_game_mask, 359f32, 0f32, WHITE);
        // No penalty shot, black or white timeouts in overtime
        match self.last_snapshot_timeout {
            TimeoutSnapshot::Ref(_) => {
                draw_texture_both!(
                    self.textures.referee_timout_graphic,
                    timeout_offset + 380f32,
                    35f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                );
                draw_text_both_ex!(
                    "REFEREE",
                    475f32 + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
                draw_text_both_ex!(
                    "TIMEOUT",
                    480f32 + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::PenaltyShot(_) => {
                draw_texture_both!(
                    self.textures.penalty_graphic,
                    timeout_offset + 380f32,
                    35f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                );
                draw_text_both_ex!(
                    "PENALTY",
                    475f32 + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
                draw_text_both_ex!(
                    "SHOT",
                    490f32 + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
            }
            _ => {}
        }
        if state.white_flag == None {
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
                    color: Color::from_rgba(0, 0, 0, 255,), // don't fade out team name if flags aren't available
                    ..Default::default()
                },
                TextParams {
                    font: self.textures.font,
                    font_size: 20,
                    color: Color::from_rgba(255, 255, 255, 255,), // don't fade out team name if flags aren't available
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
                    color: Color::from_rgba(255, 255, 255, 255),
                    ..Default::default()
                }
            );
        }
        draw_texture_both!(
            self.textures.time_and_game_state_graphic,
            167f32,
            18f32,
            WHITE
        );
        if state.white_flag.is_some() {
            draw_rectangle(1999f32, 39f32, 70f32, 33f32, WHITE);
        }
        if state.black_flag.is_some() {
            draw_rectangle(1999f32, 75f32, 70f32, 33f32, WHITE);
        }
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
            230f32 + x_off,
            95f32,
            TextParams {
                font: self.textures.font,
                font_size: 50,
                color: if [GamePeriod::SuddenDeath, GamePeriod::PreSuddenDeath]
                    .contains(&state.snapshot.current_period)
                {
                    Color::from_rgba(255, 150, 0, 255)
                } else {
                    Color::from_rgba(255, 0, 0, 255)
                },
                ..Default::default()
            },
        );
        let ot_text = match state.snapshot.current_period {
            GamePeriod::OvertimeFirstHalf => "OVERTIME 1ST HALF",
            GamePeriod::OvertimeSecondHalf => "OVERTIME 2ND HALF",
            GamePeriod::OvertimeHalfTime => "OVERTIME HALF TIME",
            GamePeriod::SuddenDeath => "SUDDEN DEATH",
            GamePeriod::PreSuddenDeath => "PRE SUDDEN DEATH",
            _ => "PRE OVERTIME",
        };
        let x_off = center_text_offset!(100f32, ot_text, 20, self.textures.font);
        draw_text_ex(
            ot_text,
            220f32 + x_off,
            45f32,
            TextParams {
                font: self.textures.font,
                font_size: 20,
                color: if [GamePeriod::SuddenDeath, GamePeriod::PreSuddenDeath]
                    .contains(&state.snapshot.current_period)
                {
                    Color::from_rgba(255, 150, 0, 255)
                } else {
                    Color::from_rgba(255, 0, 0, 255)
                },
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
                color: WHITE,
                ..Default::default()
            }
        );
    }
}
