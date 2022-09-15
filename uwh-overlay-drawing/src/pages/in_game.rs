use super::center_text_offset;
use super::Interpolate;
use super::PageRenderer;
use crate::State;
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
use uwh_common::game_snapshot::TimeoutSnapshot;

impl PageRenderer {
    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        // animate the state and time graphic to the left at 895 secs (5 seconds since period started)
        let (position_offset, alpha_offset) = if state.snapshot.secs_in_period < 1 {
            // reset animation counters if page is nearing termination
            self.secondary_animation_counter = 0f32;
            self.animation_counter = 0f32;
            (
                (0f32, -200f32).interpolate_linear(1f32),
                (255f32, 0f32).interpolate_linear(1f32) as u8,
            )
        } else if state.snapshot.current_period == GamePeriod::FirstHalf {
            self.animation_counter += 1f32 / 60f32;
            if self.animation_counter <= 5f32 {
                (
                    (0f32, -200f32).interpolate_linear(0f32),
                    (255f32, 0f32).interpolate_linear(0f32) as u8,
                )
            } else if self.animation_counter > 5f32 && self.animation_counter < 6f32 {
                (
                    (0f32, -200f32).interpolate_linear(self.animation_counter - 5f32),
                    (255f32, 0f32).interpolate_linear(self.animation_counter - 5f32) as u8,
                )
            } else {
                (
                    (0f32, -200f32).interpolate_linear(1f32),
                    (255f32, 0f32).interpolate_linear(1f32) as u8,
                )
            }
        } else {
            self.animation_counter += 1f32 / 60f32;
            match self.animation_counter {
                x if (..=1f32).contains(&x) => (
                    (0f32, -200f32).interpolate_linear(1f32 - self.animation_counter),
                    (255f32, 0f32).interpolate_linear(1f32 - self.animation_counter) as u8,
                ),
                x if (1f32..=5f32).contains(&x) => (
                    (0f32, -200f32).interpolate_linear(0f32),
                    (255f32, 0f32).interpolate_linear(0f32) as u8,
                ),
                x if (5f32..=6f32).contains(&x) => (
                    (0f32, -200f32).interpolate_linear(self.animation_counter - 5f32),
                    (255f32, 0f32).interpolate_linear(self.animation_counter - 5f32) as u8,
                ),
                _ => (
                    (0f32, -200f32).interpolate_linear(1f32),
                    (255f32, 0f32).interpolate_linear(1f32) as u8,
                ),
            }
        };
        draw_texture(self.textures.team_bar_graphic, 0_f32, 0f32, WHITE);
        draw_texture(
            self.textures.in_game_mask,
            200_f32 + position_offset,
            0f32,
            WHITE,
        );
        if state.snapshot.timeout != TimeoutSnapshot::None {
            if self.last_timeout == TimeoutSnapshot::None {
                // if this is a new timeout period
                self.secondary_animation_counter = 1f32;
            }
            if self.secondary_animation_counter > 0f32 {
                self.secondary_animation_counter -= 1f32 / 60f32;
            }
            self.last_timeout = state.snapshot.timeout;
        } else if self.last_timeout != TimeoutSnapshot::None {
            // if a timeout period just finished
            if self.secondary_animation_counter > 1f32 {
                self.secondary_animation_counter = 0f32;
                self.last_timeout = TimeoutSnapshot::None;
            } else {
                self.secondary_animation_counter += 1f32 / 60f32;
            }
        }
        let timeout_offset = (0f32, -200f32).interpolate_linear(self.secondary_animation_counter);
        let timeout_alpha_offset =
            (0f32, 255f32).interpolate_exponential_end(1f32 - self.secondary_animation_counter);
        match self.last_timeout {
            // draw text for each type of penalty
            TimeoutSnapshot::Ref(_) => {
                draw_texture(
                    self.textures.referee_timout_graphic,
                    position_offset + timeout_offset,
                    0f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                );
                draw_text_ex(
                    "REFEREE",
                    675f32 + position_offset + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    "TIMEOUT",
                    680f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
            }
            TimeoutSnapshot::White(time) => {
                draw_texture(
                    self.textures.white_timout_graphic,
                    position_offset + timeout_offset,
                    0f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                );
                draw_text_ex(
                    "WHITE",
                    675f32 + position_offset + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    "TIMEOUT",
                    665f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    format!(
                        "{}",
                        if time < 10 {
                            format!("0{}", time)
                        } else {
                            format!("{}", time)
                        }
                    )
                    .as_str(),
                    765f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 50,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
            }
            TimeoutSnapshot::Black(time) => {
                draw_texture(
                    self.textures.black_timout_graphic,
                    position_offset + timeout_offset,
                    0f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                );
                draw_text_ex(
                    "BLACK",
                    675f32 + position_offset + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    "TIMEOUT",
                    665f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    format!(
                        "{}",
                        if time < 10 {
                            format!("0{}", time)
                        } else {
                            format!("{}", time)
                        }
                    )
                    .as_str(),
                    765f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 50,
                        color: Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                        ..Default::default()
                    },
                );
            }
            TimeoutSnapshot::PenaltyShot(_) => {
                draw_texture(
                    self.textures.penalty_graphic,
                    position_offset + timeout_offset,
                    0f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8),
                );
                draw_text_ex(
                    "PENALTY",
                    675f32 + position_offset + timeout_offset,
                    67f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
                draw_text_ex(
                    "SHOT",
                    690f32 + position_offset + timeout_offset,
                    95f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 20,
                        color: if self.is_alpha_mode {
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset as u8)
                        } else {
                            Color::from_rgba(0, 0, 0, timeout_alpha_offset as u8)
                        },
                        ..Default::default()
                    },
                );
            }
            TimeoutSnapshot::None => {} // this is ugly. `TimeoutSnapshot` must be made an `Option`
        }

        draw_text_ex(
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
                    if self.is_alpha_mode { 255 } else { 0 },
                    if self.is_alpha_mode { 255 } else { 0 },
                    if self.is_alpha_mode { 255 } else { 0 },
                    if state.white_flag.is_some() {
                        alpha_offset
                    } else {
                        255
                    },
                ), // don't fade out team name if flags aren't available
                ..Default::default()
            },
        );
        draw_text_ex(
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
            },
        );
        if self.is_alpha_mode {
            draw_texture(
                self.textures.time_and_game_state_graphic,
                position_offset,
                0f32,
                WHITE,
            );
            if state.white_flag.is_some() {
                draw_rectangle(79f32, 39f32, 70f32, 33f32, WHITE);
            }
            if state.black_flag.is_some() {
                draw_rectangle(79f32, 75f32, 70f32, 33f32, WHITE);
            }
        } else {
            draw_texture(
                self.textures.time_and_game_state_graphic,
                position_offset,
                0f32,
                WHITE,
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
        }
        draw_text_ex(
            state.snapshot.b_score.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: self.textures.font,
                font_size: 30,
                ..Default::default()
            },
        );
        draw_text_ex(
            state.snapshot.w_score.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: self.textures.font,
                font_size: 30,
                color: if self.is_alpha_mode { WHITE } else { BLACK },
                ..Default::default()
            },
        );
    }
}
