use super::center_text_offset;
use super::draw_texture_both;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::State;
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::Color;

impl PageRenderer {
    /// Roster screen, displayed between 150 and 30 seconds before the next game.
    pub fn roster(&mut self, state: &State) {
        let offset = if state.snapshot.secs_in_period == 150 {
            (0f32, -650f32).interpolate_linear(
                Instant::now()
                    .duration_since(self.animation_register1)
                    .as_f64() as f32,
            )
        } else {
            self.animation_register1 = Instant::now();
            (0f32, -650f32).interpolate_linear(1f32)
        };
        draw_texture_both!(self.textures.atlantis_logo_graphic, 823f32, 712f32, WHITE);
        draw_texture_both!(self.textures.bottom_graphic, 822f32, 977f32, WHITE);
        for (i, player_identifier) in state
            .white
            .players
            .iter()
            .map(|player| (player, Color::White))
            .enumerate()
            .chain(
                state
                    .black
                    .players
                    .iter()
                    .map(|player| (player, Color::Black))
                    .enumerate(),
            )
        {
            if 60f32 * i as f32 + 220f32 > 650f32 + offset + 100f32 {
                if player_identifier.1 == Color::White {
                    draw_texture_both!(
                        self.textures.team_white_graphic,
                        if player_identifier.1 == Color::White {
                            150f32
                        } else {
                            1090f32
                        },
                        60f32 * i as f32 + 220f32,
                        WHITE
                    );
                } else {
                    draw_texture_both!(
                        self.textures.team_black_graphic,
                        if player_identifier.1 == Color::White {
                            150f32
                        } else {
                            1090f32
                        },
                        60f32 * i as f32 + 220f32,
                        WHITE
                    );
                }
                draw_text_ex(
                    format!("#{}", player_identifier.0 .1).as_str(),
                    if player_identifier.1 == Color::White {
                        185f32
                    } else {
                        1120f32
                    },
                    252f32 + 60f32 * i as f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 35,
                        color: if player_identifier.1 == Color::White {
                            BLACK
                        } else {
                            WHITE
                        },
                        ..Default::default()
                    },
                );
                draw_text_both_ex!(
                    player_identifier.0 .0.as_str(),
                    if player_identifier.1 == Color::White {
                        285f32
                    } else {
                        1220f32
                    },
                    252f32 + 60f32 * i as f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 35,
                        color: if player_identifier.1 == Color::White {
                            BLACK
                        } else {
                            WHITE
                        },
                        ..Default::default()
                    },
                    TextParams {
                        font: self.textures.font,
                        font_size: 35,
                        color: WHITE,
                        ..Default::default()
                    }
                );
            }
        }
        draw_texture_both!(
            self.textures.team_information_graphic,
            130f32,
            710f32 + offset,
            WHITE
        );
        let x_off = center_text_offset!(
            200f32,
            state.black.team_name.to_uppercase().as_str(),
            45,
            self.textures.font
        );
        draw_text_both!(
            state.black.team_name.to_uppercase().as_str(),
            1350f32 + x_off,
            805f32 + offset,
            TextParams {
                font: self.textures.font,
                font_size: 45,
                ..Default::default()
            }
        );
        let x_off = center_text_offset!(
            200f32,
            state.black.team_name.to_uppercase().as_str(),
            45,
            self.textures.font
        );
        draw_text_both_ex!(
            state.white.team_name.to_uppercase().as_str(),
            120f32 + x_off,
            805f32 + offset,
            TextParams {
                font: self.textures.font,
                font_size: 50,
                color: BLACK,
                ..Default::default()
            },
            TextParams {
                font: self.textures.font,
                font_size: 50,
                color: WHITE,
                ..Default::default()
            }
        );
        if state.white_flag.is_some() {
            draw_rectangle(2500f32, 738f32 + offset, 180f32, 100f32, WHITE);
        }
        if state.black_flag.is_some() {
            draw_rectangle(3083f32, 738f32 + offset, 180f32, 100f32, WHITE);
        }
        if let Some(flag) = state.white_flag {
            draw_texture_ex(
                flag,
                580f32,
                738f32 + offset,
                WHITE,
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
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(180f32, 100f32)),
                    ..Default::default()
                },
            );
        }
        let x_off = center_text_offset!(
            135f32,
            format!("GAME ID: {}", &state.game_id.to_string()).as_str(),
            25,
            self.textures.font
        );
        draw_text_ex(
            format!("GAME ID: {}", &state.game_id.to_string()).as_str(),
            830f32 + x_off,
            745f32 + offset,
            TextParams {
                font: self.textures.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let x_off = center_text_offset!(124f32, state.start_time.as_str(), 25, self.textures.font);
        draw_text_ex(
            state.start_time.as_str(),
            838f32 + x_off,
            780f32 + offset,
            TextParams {
                font: self.textures.font,
                font_size: 25,
                ..Default::default()
            },
        );
        let x_off = center_text_offset!(
            110f32,
            format!("POOL: {}", &state.pool.to_string()).as_str(),
            25,
            self.textures.font
        );
        draw_text_ex(
            format!("POOL: {}", &state.pool.to_string()).as_str(),
            855f32 + x_off,
            815f32 + offset,
            TextParams {
                font: self.textures.font,
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
    }
}
