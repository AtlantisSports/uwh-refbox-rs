use super::{
    Interpolate, PageRenderer, TIME_BOX_CUT, TimeoutFlag, centred_x, draw_texture_both,
    draw_widened, fit_text, widen_for,
};
use crate::{
    State,
    pages::{Justify, draw_text_both, draw_text_both_ex, draw_texture_both_ex},
};
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::{GamePeriod, TimeoutSnapshot};

impl PageRenderer {
    /// Display info during game play
    pub fn in_game_display(&mut self, state: &State) {
        let mut time = Instant::now()
            .duration_since(self.animation_register2)
            .as_f64() as f32;
        let (timeout_offset, timeout_alpha_offset) = if state.snapshot.timeout.is_some() {
            if self.last_snapshot_timeout.is_none() {
                // if this is a new timeout period
                self.animation_register2 = Instant::now();
                time = 0.0f32;
            }
            self.last_snapshot_timeout = state.snapshot.timeout;
            if time < 1f32 {
                (
                    (0f32, -270f32).interpolate_linear(1f32 - time),
                    (0f32, 1f32).interpolate_exponential_end(time),
                )
            } else {
                (
                    (0f32, -270f32).interpolate_linear(0f32),
                    (0f32, 1f32).interpolate_exponential_end(1f32),
                )
            }
        } else if self.last_snapshot_timeout.is_some() {
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
                self.last_snapshot_timeout = None;
                (
                    (0f32, -270f32).interpolate_linear(1f32),
                    (0f32, 1f32).interpolate_exponential_end(0f32),
                )
            } else {
                (
                    (0f32, -270f32).interpolate_linear(time),
                    (0f32, 1f32).interpolate_exponential_end(1f32 - time),
                )
            }
        } else {
            // return any values when both are None, cause we won't be redering anyways
            (
                (0f32, -270f32).interpolate_linear(0f32),
                (0f32, 1f32).interpolate_exponential_end(1f32),
            )
        };
        // The time box widens from its centre for a longer period name, and the flag behind it
        // moves right by as much as the box's right end does.
        let period = state.lang.period(state.snapshot.current_period);
        let box_extra = widen_for(
            measure_text(period, Some(&self.assets.font), 20, 1.0).width,
            180f32,
        );
        if let Some(timeout) = self.last_snapshot_timeout {
            let fade = |color: Color| Color {
                a: timeout_alpha_offset,
                ..color
            };
            let (tex, lines, english, english_x, text_color) = match timeout {
                TimeoutSnapshot::Ref(_) => (
                    &self.assets.referee_timout,
                    state.lang.referee_timeout(),
                    ("REFEREE", "TIMEOUT"),
                    (675f32, 680f32),
                    BLACK,
                ),
                TimeoutSnapshot::White(_) => (
                    &self.assets.white_timout,
                    state.lang.team_timeout(false),
                    ("WHITE", "TIMEOUT"),
                    (675f32, 665f32),
                    BLACK,
                ),
                TimeoutSnapshot::Black(_) => (
                    &self.assets.black_timout,
                    state.lang.team_timeout(true),
                    ("BLACK", "TIMEOUT"),
                    (675f32, 665f32),
                    WHITE,
                ),
                TimeoutSnapshot::PenaltyShot(_) => (
                    &self.assets.penalty,
                    state.lang.penalty_shot(),
                    ("PENALTY", "SHOT"),
                    (675f32, 690f32),
                    BLACK,
                ),
            };
            let flag = TimeoutFlag {
                font: &self.assets.font,
                lines,
                english,
                english_x,
                slide: timeout_offset,
                shift: box_extra / 2f32,
            };
            flag.draw_art(tex, fade(WHITE));
            match timeout {
                TimeoutSnapshot::White(time) | TimeoutSnapshot::Black(time) => {
                    // A team's flag shows its words only once it is far enough out.
                    if timeout_offset > -175f32 {
                        flag.draw_text(fade(text_color));
                    }
                    let params = |color: Color| TextParams {
                        font: Some(&self.assets.font),
                        font_size: 50,
                        color,
                        ..Default::default()
                    };
                    draw_text_both_ex!(
                        format!("{time}").as_str(),
                        773f32 + flag.after_text(),
                        90f32,
                        params(fade(text_color)),
                        params(fade(WHITE))
                    );
                }
                TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
                    flag.draw_text(fade(text_color));
                }
            }
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
                font: Some(&self.assets.font),
                font_size: 20,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
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
                font: Some(&self.assets.font),
                font_size: 20,
                color: WHITE,
                ..Default::default()
            }
        );
        draw_widened(
            &self.assets.time_and_game_state,
            367f32,
            18f32,
            TIME_BOX_CUT,
            box_extra,
            WHITE,
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
            423f32 + x_off,
            67f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 50,
                ..Default::default()
            },
        );
        // Centred where the 180 px period field is centred; a longer name widens the box instead.
        draw_text_ex(
            period,
            centred_x(&self.assets.font, period, 20, 513f32),
            100f32,
            TextParams {
                font: Some(&self.assets.font),
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
            state.snapshot.scores.black.to_string().as_str(),
            40f32,
            104f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 30,
                ..Default::default()
            }
        );
        draw_text_both_ex!(
            state.snapshot.scores.white.to_string().as_str(),
            40f32,
            65f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 30,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
                font_size: 30,
                color: WHITE,
                ..Default::default()
            }
        );

        if let Some(logo) = state.event_logo.as_ref() {
            let x = 1900f32 - logo.color.width();
            draw_texture_both!(logo, x, 20f32, WHITE);
        }

        let sponsor_alpha = if state.snapshot.current_period == GamePeriod::HalfTime {
            match state.snapshot.secs_in_period {
                32.. => (0f32, 1f32).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register4)
                        .as_f64() as f32,
                ),
                31 => {
                    self.animation_register4 = Instant::now();
                    1f32
                }
                0..=30 => (1f32, 0f32).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register4)
                        .as_f64() as f32,
                ),
            }
        } else {
            self.animation_register4 = Instant::now();
            0f32
        };

        if state.snapshot.current_period == GamePeriod::HalfTime {
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
        }
    }
}
