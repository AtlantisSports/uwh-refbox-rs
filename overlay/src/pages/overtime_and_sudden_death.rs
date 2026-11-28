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

/// The room the period name has in the time box as drawn: the longest English name
/// ("OVERTIME HALF TIME") takes 191 px of it.
const PERIOD_ROOM: f32 = 192f32;

impl PageRenderer {
    /// Display during overtime. Has no animations
    pub fn overtime_and_sudden_death_display(&mut self, state: &State) {
        let mut time = Instant::now()
            .duration_since(self.animation_register2)
            .as_f64() as f32;
        // animate the state and time graphic to the left at 895 secs (5 seconds since period started)
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
        let ot_text = match state.snapshot.current_period {
            period @ (GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::SuddenDeath
            | GamePeriod::PreSuddenDeath) => state.lang.period(period),
            _ => state.lang.period(GamePeriod::PreOvertime),
        };
        let box_extra = widen_for(
            measure_text(ot_text, Some(&self.assets.font), 20, 1.0).width,
            PERIOD_ROOM,
        );

        // No black or white timeouts in overtime
        if let Some(timeout) = self.last_snapshot_timeout {
            let flag = match timeout {
                TimeoutSnapshot::Ref(_) => Some((
                    &self.assets.referee_timout,
                    state.lang.referee_timeout(),
                    ("REFEREE", "TIMEOUT"),
                    (675f32, 680f32),
                )),
                TimeoutSnapshot::PenaltyShot(_) => Some((
                    &self.assets.penalty,
                    state.lang.penalty_shot(),
                    ("PENALTY", "SHOT"),
                    (675f32, 690f32),
                )),
                _ => None,
            };
            if let Some((tex, lines, english, english_x)) = flag {
                let flag = TimeoutFlag {
                    font: &self.assets.font,
                    lines,
                    english,
                    english_x,
                    slide: timeout_offset,
                    shift: box_extra / 2f32,
                };
                flag.draw_art(
                    tex,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    },
                );
                flag.draw_text(Color {
                    a: timeout_alpha_offset,
                    ..BLACK
                });
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
                color: BLACK, // don't fade out team name if flags aren't available
                ..Default::default()
            },
            TextParams {
                font: Some(&self.assets.font),
                font_size: 20,
                color: WHITE, // don't fade out team name if flags aren't available
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
            430f32 + x_off,
            95f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 50,
                color: if [GamePeriod::SuddenDeath, GamePeriod::PreSuddenDeath]
                    .contains(&state.snapshot.current_period)
                {
                    GOLD
                } else {
                    RED
                },
                ..Default::default()
            },
        );
        // Centred where the 200 px period field is centred; a longer name widens the box instead.
        draw_text_ex(
            ot_text,
            centred_x(&self.assets.font, ot_text, 20, 520f32),
            45f32,
            TextParams {
                font: Some(&self.assets.font),
                font_size: 20,
                color: if [GamePeriod::SuddenDeath, GamePeriod::PreSuddenDeath]
                    .contains(&state.snapshot.current_period)
                {
                    GOLD
                } else {
                    RED
                },
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
    }
}
