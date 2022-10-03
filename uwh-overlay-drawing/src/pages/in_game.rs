use super::center_text_offset;
use super::draw_texture_both;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::State;
use crate::ALPHA_MAX;
use crate::ALPHA_MIN;
use crate::TIME_AND_STATE_SHRINK_FROM;
use crate::TIME_AND_STATE_SHRINK_TO;
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
use uwh_common::game_snapshot::TimeoutSnapshot;

impl PageRenderer {
    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        // animate the state and time graphic 5 seconds since period started)
        let (position_offset, alpha_offset) = if state.snapshot.secs_in_period < 1 {
            // reset animation counters if page is nearing termination
            self.animation_register1 = Instant::now();
            self.animation_register2 = Instant::now();
            if state.snapshot.current_period == GamePeriod::HalfTime {
                (
                    (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO).interpolate_linear(0f32),
                    (ALPHA_MAX, ALPHA_MIN).interpolate_linear(0f32) as u8,
                )
            } else {
                (
                    (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO).interpolate_linear(1f32),
                    (ALPHA_MAX, ALPHA_MIN).interpolate_linear(1f32) as u8,
                )
            }
        } else if state.snapshot.current_period == GamePeriod::FirstHalf {
            let time = Instant::now()
                .duration_since(self.animation_register1)
                .as_f64();
            match time {
                x if (..=5f64).contains(&x) => (
                    (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO).interpolate_linear(0f32),
                    (ALPHA_MAX, ALPHA_MIN).interpolate_linear(0f32) as u8,
                ),
                x if (5f64..=6f64).contains(&x) => (
                    (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                        .interpolate_linear(time as f32 - 5f32),
                    (ALPHA_MAX, ALPHA_MIN).interpolate_linear(time as f32 - 5f32) as u8,
                ),
                _ => (
                    (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO).interpolate_linear(1f32),
                    (ALPHA_MAX, ALPHA_MIN).interpolate_linear(1f32) as u8,
                ),
            }
        } else {
            let time = Instant::now()
                .duration_since(self.animation_register1)
                .as_f64();
            match time {
                x if (..=1f64).contains(&x) => {
                    if state.snapshot.current_period == GamePeriod::SecondHalf {
                        (
                            (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                                .interpolate_linear(0f32),
                            (ALPHA_MAX, ALPHA_MIN).interpolate_linear(0f32) as u8,
                        )
                    } else {
                        (
                            (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                                .interpolate_linear(1f32 - time as f32),
                            (ALPHA_MAX, ALPHA_MIN).interpolate_linear(1f32 - time as f32) as u8,
                        )
                    }
                }
                x if (1f64..=5f64).contains(&x) => (
                    (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO).interpolate_linear(0f32),
                    (ALPHA_MAX, ALPHA_MIN).interpolate_linear(0f32) as u8,
                ),
                x if (5f64..=6f64).contains(&x) => {
                    if state.snapshot.current_period == GamePeriod::HalfTime {
                        (
                            (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                                .interpolate_linear(0f32),
                            (ALPHA_MAX, ALPHA_MIN).interpolate_linear(0f32) as u8,
                        )
                    } else {
                        (
                            (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                                .interpolate_linear(time as f32 - 5f32),
                            (ALPHA_MAX, ALPHA_MIN).interpolate_linear(time as f32 - 5f32) as u8,
                        )
                    }
                }
                _ => {
                    if state.snapshot.current_period == GamePeriod::HalfTime {
                        (
                            (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                                .interpolate_linear(0f32),
                            (ALPHA_MAX, ALPHA_MIN).interpolate_linear(0f32) as u8,
                        )
                    } else {
                        (
                            (TIME_AND_STATE_SHRINK_FROM, TIME_AND_STATE_SHRINK_TO)
                                .interpolate_linear(1f32),
                            (ALPHA_MAX, ALPHA_MIN).interpolate_linear(1f32) as u8,
                        )
                    }
                }
            }
        };
        draw_texture_both!(self.textures.team_bar_graphic, 26f32, 37f32, WHITE);
        draw_texture_both!(
            self.textures.in_game_mask,
            580f32 + position_offset,
            37f32,
            WHITE
        );
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
                        (0f32, 255f32).interpolate_exponential_end(time),
                    )
                } else {
                    (
                        (0f32, -200f32).interpolate_linear(0f32),
                        (0f32, 255f32).interpolate_exponential_end(1f32),
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
                        (0f32, 255f32).interpolate_exponential_end(0f32),
                    )
                } else {
                    (
                        (0f32, -200f32).interpolate_linear(time),
                        (0f32, 255f32).interpolate_exponential_end(1f32 - time),
                    )
                }
            } else {
                // return any values when both are None, cause we won't be redering anyways
                (
                    (0f32, -200f32).interpolate_linear(0f32),
                    (0f32, 255f32).interpolate_exponential_end(1f32),
                )
            };
        match self.last_snapshot_timeout {
            // draw text for each type of penalty
            TimeoutSnapshot::Ref(_) => {
                draw_texture_both!(
                    self.textures.referee_timout_graphic,
                    position_offset + timeout_offset + 580f32,
                    35f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                );
                draw_text_both_ex!(
                    "REFEREE",
                    675f32 + position_offset + timeout_offset,
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
                    680f32 + position_offset + timeout_offset,
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
            TimeoutSnapshot::White(time) => {
                draw_texture_both!(
                    self.textures.white_timout_graphic,
                    position_offset + timeout_offset + 580f32,
                    35f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                );
                draw_text_both_ex!(
                    "WHITE",
                    675f32 + position_offset + timeout_offset,
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
                    665f32 + position_offset + timeout_offset,
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
                draw_text_both_ex!(
                    format!("{time}").as_str(),
                    773f32 + position_offset + timeout_offset,
                    90f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 50,
                        color: Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                    TextParams {
                        font: self.textures.font,
                        font_size: 50,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::Black(time) => {
                draw_texture_both!(
                    self.textures.black_timout_graphic,
                    position_offset + timeout_offset + 580f32,
                    35f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                );
                draw_text_both!(
                    "BLACK",
                    675f32 + position_offset + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
                draw_text_both!(
                    "TIMEOUT",
                    665f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
                draw_text_both!(
                    format!("{time}").as_str(),
                    773f32 + position_offset + timeout_offset,
                    90f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 50,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    }
                );
            }
            TimeoutSnapshot::PenaltyShot(_) => {
                draw_texture_both!(
                    self.textures.penalty_graphic,
                    position_offset + timeout_offset + 580f32,
                    35f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                );
                draw_text_both_ex!(
                    "PENALTY",
                    675f32 + position_offset + timeout_offset,
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
                    690f32 + position_offset + timeout_offset,
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
            TimeoutSnapshot::None => {} // this is ugly. `TimeoutSnapshot` must be made an `Option`
        }

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
                color: Color::from_rgba(
                    0,
                    0,
                    0,
                    if state.white_flag.is_some() {
                        alpha_offset
                    } else {
                        255
                    },
                ), // don't fade out team name if flags aren't available
                ..Default::default()
            },
            TextParams {
                font: self.textures.font,
                font_size: 20,
                color: Color::from_rgba(
                    255,
                    255,
                    255,
                    if state.white_flag.is_some() {
                        alpha_offset
                    } else {
                        255
                    },
                ), // don't fade out team name if flags aren't available
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
                color: Color::from_rgba(
                    255,
                    255,
                    255,
                    if state.black_flag.is_some() {
                        alpha_offset
                    } else {
                        255
                    },
                ),
                ..Default::default()
            }
        );
        draw_texture_both!(
            self.textures.time_and_game_state_graphic,
            position_offset + 367f32,
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
            430f32 + position_offset + x_off,
            67f32,
            TextParams {
                font: self.textures.font,
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
            478f32 + position_offset,
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
