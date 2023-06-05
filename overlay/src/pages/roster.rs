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
use uwh_common::game_snapshot::Color as UwhColor;

impl PageRenderer {
    /// Roster screen, displayed between 150 and 30 seconds before the next game.
    pub fn roster(&mut self, state: &State) {
        if state.snapshot.secs_in_period >= 145 {
            let offset = if state.snapshot.secs_in_period == 150 {
                (0f32, -650f32).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register1)
                        .as_f64() as f32,
                )
            } else {
                if state.snapshot.secs_in_period != 145 {
                    self.animation_register1 = Instant::now();
                }
                (0f32, -650f32).interpolate_linear(1f32)
            };
            let timeout_alpha_offset = if state.snapshot.secs_in_period == 145 {
                self.animation_register2 = Instant::now();
                (ALPHA_MAX, ALPHA_MIN).interpolate_linear(
                    Instant::now()
                        .duration_since(self.animation_register1)
                        .as_f64() as f32,
                ) as u8
            } else {
                255
            };
            if let Some(logo) = self.assets.tournament_logo.as_ref() {
                if offset <= -210f32 {
                    let x = (1920f32 - logo.color.width()) / 2f32;
                    draw_texture_both!(
                        logo,
                        x,
                        500f32,
                        Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                    );
                }
            }

            draw_texture_both!(
                self.assets.atlantis_logo,
                836f32,
                725f32,
                Color::from_rgba(255, 255, 255, timeout_alpha_offset)
            );
            draw_texture_both!(
                self.assets.bottom,
                822f32,
                977f32,
                Color::from_rgba(255, 255, 255, timeout_alpha_offset)
            );
            for (i, player_identifier) in state
                .white
                .players
                .iter()
                .map(|player| (player, UwhColor::White))
                .enumerate()
                .chain(
                    state
                        .black
                        .players
                        .iter()
                        .map(|player| (player, UwhColor::Black))
                        .enumerate(),
                )
            {
                if 60f32 * i as f32 + 220f32 > 650f32 + offset + 100f32 {
                    if player_identifier.1 == UwhColor::White {
                        draw_texture_both!(
                            self.assets.team_white_banner,
                            if player_identifier.1 == UwhColor::White {
                                150f32
                            } else {
                                1090f32
                            },
                            60f32 * i as f32 + 220f32,
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                        );
                    } else {
                        draw_texture_both!(
                            self.assets.team_black_banner,
                            if player_identifier.1 == UwhColor::White {
                                150f32
                            } else {
                                1090f32
                            },
                            60f32 * i as f32 + 220f32,
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                        );
                    }
                    draw_text_ex(
                        format!("#{}", player_identifier.0 .1).as_str(),
                        if player_identifier.1 == UwhColor::White {
                            185f32
                        } else {
                            1120f32
                        },
                        252f32 + 60f32 * i as f32,
                        TextParams {
                            font: self.assets.font,
                            font_size: 35,
                            color: if player_identifier.1 == UwhColor::White {
                                Color::from_rgba(0, 0, 0, timeout_alpha_offset)
                            } else {
                                Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                            },
                            ..Default::default()
                        },
                    );
                    draw_text_both_ex!(
                        player_identifier.0 .0.as_str(),
                        if player_identifier.1 == UwhColor::White {
                            285f32
                        } else {
                            1220f32
                        },
                        252f32 + 60f32 * i as f32,
                        TextParams {
                            font: self.assets.font,
                            font_size: 35,
                            color: if player_identifier.1 == UwhColor::White {
                                Color::from_rgba(0, 0, 0, timeout_alpha_offset)
                            } else {
                                Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                            },
                            ..Default::default()
                        },
                        TextParams {
                            font: self.assets.font,
                            font_size: 35,
                            color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                            ..Default::default()
                        }
                    );
                }
            }
            draw_texture_both!(
                self.assets.team_information,
                130f32,
                710f32 + offset,
                Color::from_rgba(255, 255, 255, timeout_alpha_offset)
            );
            let (x_off, text) = center_text_offset!(
                217f32,
                state.black.team_name.to_uppercase().as_str(),
                45,
                self.assets.font
            );
            draw_text_both!(
                text.as_str(),
                1350f32 + x_off,
                805f32 + offset,
                TextParams {
                    font: self.assets.font,
                    font_size: 45,
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
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
                805f32 + offset,
                TextParams {
                    font: self.assets.font,
                    font_size: 45,
                    color: Color::from_rgba(0, 0, 0, timeout_alpha_offset),
                    ..Default::default()
                },
                TextParams {
                    font: self.assets.font,
                    font_size: 45,
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                    ..Default::default()
                }
            );
            if state.white_flag.is_some() {
                draw_rectangle(
                    2500f32,
                    738f32 + offset,
                    180f32,
                    100f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                );
            }
            if state.black_flag.is_some() {
                draw_rectangle(
                    3083f32,
                    738f32 + offset,
                    180f32,
                    100f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                );
            }
            if let Some(flag) = state.white_flag {
                draw_texture_ex(
                    flag,
                    580f32,
                    738f32 + offset,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                    DrawTextureParams {
                        dest_size: Some(vec2(180f32, 100f32)),
                        ..Default::default()
                    },
                );
            }
            if let Some(flag) = state.black_flag {
                draw_texture_ex(
                    flag,
                    1163f32,
                    738f32 + offset,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset),
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
                745f32 + offset,
                TextParams {
                    font: self.assets.font,
                    font_size: 25,
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                    ..Default::default()
                },
            );
            let (x_off, text) =
                center_text_offset!(124f32, state.start_time.as_str(), 25, self.assets.font);
            draw_text_ex(
                text.as_str(),
                838f32 + x_off,
                780f32 + offset,
                TextParams {
                    font: self.assets.font,
                    font_size: 25,
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                    ..Default::default()
                },
            );
            let (x_off, text) =
                center_text_offset!(110f32, &state.pool.to_string(), 25, self.assets.font);
            draw_text_ex(
                text.as_str(),
                855f32 + x_off,
                815f32 + offset,
                TextParams {
                    font: self.assets.font,
                    font_size: 25,
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
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
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                    ..Default::default()
                },
            );
            draw_text_ex(
                "NEXT GAME",
                905f32,
                1044f32,
                TextParams {
                    font: self.assets.font,
                    font_size: 20,
                    color: Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                    ..Default::default()
                },
            );
        } else {
            let white_to_black_point = (state.white.players.len() / 4 * 4) as f32 + 0.5;
            let black_to_red_point =
                white_to_black_point + (state.black.players.len() / 4 * 4) as f32 + 0.5;
            let red_fade_point = black_to_red_point; //(state.black.players.len() / 4 * 4) as f32 + 0.5;
            match Instant::now()
                .duration_since(self.animation_register1)
                .as_f64() as f32
            {
                a if (0f32..=white_to_black_point).contains(&a) => {
                    draw_texture_both!(self.assets.white_team_name_bg_rpd, 130f32, 710f32, WHITE);
                }
                a if (white_to_black_point..=black_to_red_point).contains(&a) => {
                    draw_texture_both!(self.assets.black_team_name_bg_rpd, 130f32, 710f32, WHITE);
                }
                a if (black_to_red_point..=red_fade_point).contains(&a) => {
                    draw_texture_both!(self.assets.red_team_name_bg_rpd, 130f32, 710f32, WHITE);
                }
                _ => {}
            }
        }
    }
}
