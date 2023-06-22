use super::center_text_offset;
use super::draw_texture_both;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::pages::get_input;
use crate::pages::multilinify;
use crate::State;
use crate::BYTE_MAX;
use crate::BYTE_MIN;
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::Color as UwhColor;

const RPD_CARD_TIME: usize = 5;
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
                (BYTE_MAX, BYTE_MIN).interpolate_linear(
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
            if state.white.flag.is_some() {
                draw_rectangle(
                    2500f32,
                    738f32 + offset,
                    180f32,
                    100f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                );
            }
            if state.black.flag.is_some() {
                draw_rectangle(
                    3083f32,
                    738f32 + offset,
                    180f32,
                    100f32,
                    Color::from_rgba(255, 255, 255, timeout_alpha_offset),
                );
            }
            if let Some(flag) = state.white.flag {
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
            if let Some(flag) = state.black.flag {
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
            // Page Transition Points:
            // p1: Team White Players -> Team White Support
            // p2: Team White Support -> Team Black Players
            // p3: Team Black Players -> Team Black Support
            // p4: Team Black Support -> Referees
            // p5: Referees -> Fade out
            let p1 = ((state.white.players.len() + 3) / 4 * RPD_CARD_TIME) as f32;
            let p2 = p1 + ((state.white.support_members.len() + 3) / 4 * RPD_CARD_TIME) as f32;
            let p3 = p2 + ((state.black.players.len() + 3) / 4 * RPD_CARD_TIME) as f32;
            let p4 = p3 + ((state.black.support_members.len() + 3) / 4 * RPD_CARD_TIME) as f32;
            let p5 = p4 + ((state.referees.len() + 3) / 4 * RPD_CARD_TIME) as f32;

            /// only team support has `role`
            /// only players have `geared_picture` and `number`.
            /// players, team support and referees have `picture`
            struct CardRepr<'a> {
                name: &'a str,
                role: Option<&'a str>,
                number: Option<u8>,
                geared_picture: &'a Option<Texture2D>,
                picture: &'a Option<Texture2D>,
            }

            let rpd_selector = match Instant::now()
                .duration_since(self.animation_register2)
                .as_f64() as f32
            {
                a if (0f32..=p2).contains(&a) => Some((
                    state.white.team_name.as_str(),
                    if (0f32..=p1).contains(&a) {
                        state
                            .white
                            .players
                            .iter()
                            .map(|player| CardRepr {
                                name: &player.0,
                                role: None,
                                number: Some(player.1),
                                picture: &player.2,
                                geared_picture: &player.3,
                            })
                            .collect::<Vec<_>>()
                    } else {
                        state
                            .white
                            .support_members
                            .iter()
                            .map(|supporter| CardRepr {
                                name: &supporter.0,
                                role: Some(&supporter.1),
                                number: None,
                                picture: &supporter.2,
                                geared_picture: &None,
                            })
                            .collect()
                    },
                    &self.assets.white_rpd,
                    BLACK,
                    if (0f32..=p1).contains(&a) { 0f32 } else { p1 },
                )),
                a if (p2..=p4).contains(&a) => Some((
                    state.black.team_name.as_str(),
                    if (p2..=p3).contains(&a) {
                        state
                            .black
                            .players
                            .iter()
                            .map(|player| CardRepr {
                                name: &player.0,
                                role: None,
                                number: Some(player.1),
                                picture: &player.2,
                                geared_picture: &player.3,
                            })
                            .collect()
                    } else {
                        state
                            .black
                            .support_members
                            .iter()
                            .map(|supporter| CardRepr {
                                name: &supporter.0,
                                role: Some(&supporter.1),
                                number: None,
                                picture: &supporter.2,
                                geared_picture: &None,
                            })
                            .collect()
                    },
                    &self.assets.black_rpd,
                    WHITE,
                    if (p2..=p3).contains(&a) { p2 } else { p3 },
                )),
                a if (p4..=p5).contains(&a) => Some((
                    "REFEREES",
                    state
                        .referees
                        .iter()
                        .map(|referee| CardRepr {
                            name: &referee.0,
                            role: None,
                            number: None,
                            picture: &referee.1,
                            geared_picture: &None,
                        })
                        .collect(),
                    &self.assets.red_rpd,
                    WHITE,
                    if (p3..=p4).contains(&a) { p3 } else { p4 },
                )),
                _ => None,
            };

            if let Some((team_name, card_repr, team_textures, text_color, page_start)) =
                rpd_selector
            {
                draw_texture_both!(team_textures.team_name_bg, 464f32, 80f32, WHITE);
                let (x_off, text) = center_text_offset!(
                    469f32,
                    team_name.to_uppercase().as_str(),
                    120,
                    self.assets.font
                );
                draw_text_both_ex!(
                    text.as_str(),
                    470f32 + x_off,
                    230f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 120,
                        color: text_color,
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 120,
                        color: WHITE,
                        ..Default::default()
                    }
                );

                card_repr
                    .rchunks(4)
                    .nth(
                        (Instant::now()
                            .duration_since(self.animation_register2)
                            .as_f64() as f32
                            - page_start) as usize
                            / RPD_CARD_TIME,
                    )
                    .map(|x| {
                        x.iter().enumerate().for_each(
                            |(
                                i,
                                CardRepr {
                                    name,
                                    role,
                                    number,
                                    geared_picture,
                                    picture,
                                },
                            )| {
                                draw_texture_both!(
                                    team_textures.frame_without_number,
                                    i as f32 * (473f32) + 28f32,
                                    355f32,
                                    WHITE
                                );
                                if let Some(number) = number {
                                    let card_picture = if (Instant::now()
                                        .duration_since(self.animation_register2)
                                        .as_f64()
                                        as f32
                                        - page_start)
                                        as usize
                                        % RPD_CARD_TIME
                                        < 3
                                    {
                                        picture
                                    } else {
                                        geared_picture
                                    };
                                    draw_rectangle(
                                        i as f32 * (473f32) + 43f32 + 1920f32,
                                        372f32,
                                        415f32,
                                        415f32,
                                        WHITE,
                                    );
                                    draw_texture_ex(
                                        card_picture.unwrap_or(self.assets.potrait_default.color),
                                        i as f32 * (473f32) + 43f32,
                                        372f32,
                                        WHITE,
                                        DrawTextureParams {
                                            dest_size: Some(vec2(415f32, 415f32)),
                                            ..Default::default()
                                        },
                                    );
                                    draw_texture_both!(
                                        team_textures.frame_number,
                                        i as f32 * (473f32) + 43f32,
                                        355f32,
                                        WHITE
                                    );
                                    let (x_off, text) = center_text_offset!(
                                        70f32,
                                        number.to_string(),
                                        65,
                                        self.assets.font
                                    );
                                    draw_text_both_ex!(
                                        text.as_str(),
                                        i as f32 * (473f32) + 43f32 + x_off,
                                        440f32 as f32,
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 65,
                                            color: text_color,
                                            ..Default::default()
                                        },
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 65,
                                            color: WHITE,
                                            ..Default::default()
                                        }
                                    );
                                } else {
                                    draw_rectangle(
                                        i as f32 * (473f32) + 43f32 + 1920f32,
                                        372f32,
                                        415f32,
                                        415f32,
                                        WHITE,
                                    );
                                    draw_texture_ex(
                                        picture.unwrap_or(self.assets.potrait_default.color),
                                        i as f32 * (473f32) + 43f32,
                                        372f32,
                                        WHITE,
                                        DrawTextureParams {
                                            dest_size: Some(vec2(415f32, 415f32)),
                                            ..Default::default()
                                        },
                                    );
                                }
                                if let Some(role) = role {
                                    draw_texture_both!(
                                        team_textures.team_member_role_bg,
                                        i as f32 * (473f32) + 68f32,
                                        355f32,
                                        WHITE
                                    );
                                    let (x_off, text) =
                                        center_text_offset!(160f32, role, 45, self.assets.font);
                                    draw_text_both_ex!(
                                        &text,
                                        i as f32 * (473f32) + 88f32 + x_off,
                                        405f32,
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 45,
                                            color: text_color,
                                            ..Default::default()
                                        },
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 45,
                                            color: WHITE,
                                            ..Default::default()
                                        }
                                    );
                                }

                                let lines = multilinify(name, 214f32, Some(self.assets.font), 40);
                                let text_box_texture = match lines.len() {
                                    1 => &team_textures.single_line_name_bg,
                                    2 => &team_textures.double_line_name_bg,
                                    _ => &team_textures.triple_line_name_bg,
                                };
                                draw_texture_both!(
                                    text_box_texture,
                                    i as f32 * (475f32) + 28f32,
                                    830f32,
                                    WHITE
                                );
                                for (j, line) in lines.iter().take(3).enumerate() {
                                    let (x_off, text) =
                                        center_text_offset!(214f32, line, 45, self.assets.font);
                                    draw_text_both_ex!(
                                        text.as_str(),
                                        i as f32 * (473f32) + 32f32 + x_off,
                                        885f32 + 50f32 * j as f32,
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 45,
                                            color: text_color,
                                            ..Default::default()
                                        },
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 45,
                                            color: WHITE,
                                            ..Default::default()
                                        }
                                    );
                                }
                            },
                        )
                    });
            }
        }
    }
}
