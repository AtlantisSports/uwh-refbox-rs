use super::center_text_offset;
use super::draw_texture_both;
use super::Interpolate;
use super::PageRenderer;
use crate::pages::draw_text_both;
use crate::pages::draw_text_both_ex;
use crate::pages::multilinify;
use crate::Member;
use crate::State;
use crate::BYTE_MAX;
use crate::BYTE_MIN;
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::Color as UwhColor;

const RPD_CARD_TIME: usize = 5;
const NUMBER_BG_WIDTH: f32 = 100f32;

fn rpd_groups(members: &Vec<Member>) -> impl Iterator<Item = Vec<Member>> {
    let mut n4 = members.len() / 4;
    let mut n3 = 0;
    // 1,2,5 are failing cases for this algorithm
    if [1, 2].contains(&members.len()) {
        return vec![members.clone()].into_iter(); // 1,2 get returned as is
    } else if members.len() == 5 {
        return vec![members[..3].to_vec(), members[3..].to_vec()].into_iter(); // partition 5 into 3, 2
    }
    match members.len() % 4 {
        3 => {
            n3 += 1;
        }
        2 => {
            n3 += 2;
            n4 -= 1;
        }
        1 => {
            // 1
            n3 += 3;
            n4 -= 2;
        }
        0 => {}
        _ => unreachable!(),
    }
    let mut divd = Vec::new();
    for i in 0..n4 {
        divd.push(members[i * 4..i * 4 + 4].to_vec());
    }
    for i in 0..n3 {
        divd.push(members[n4 * 4 + i * 3..n4 * 4 + i * 3 + 3].to_vec());
    }
    divd.into_iter()
}

impl PageRenderer {
    /// Roster screen, displayed between 150 and 30 seconds before the next game.
    pub fn roster(&mut self, state: &State) {
        draw_texture(self.bg, 0f32, 0f32, WHITE);
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
                .get_players()
                .map(|player| (player, UwhColor::White))
                .enumerate()
                .chain(
                    state
                        .black
                        .get_players()
                        .map(|player| (player, UwhColor::Black))
                        .enumerate(),
                )
            {
                if 60f32 * i as f32 + 220f32 > 650f32 + offset + 100f32 {
                    if player_identifier.1 == UwhColor::White {
                        draw_texture_both!(
                            self.assets.team_white_banner,
                            if player_identifier.1 == UwhColor::White {
                                130f32
                            } else {
                                1108f32
                            },
                            60f32 * i as f32 + 220f32,
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                        );
                    } else {
                        draw_texture_both!(
                            self.assets.team_black_banner,
                            if player_identifier.1 == UwhColor::White {
                                130f32
                            } else {
                                1108f32
                            },
                            60f32 * i as f32 + 220f32,
                            Color::from_rgba(255, 255, 255, timeout_alpha_offset)
                        );
                    }
                    draw_text_ex(
                        format!("#{}", player_identifier.0.number.unwrap()).as_str(),
                        if player_identifier.1 == UwhColor::White {
                            165f32
                        } else {
                            1138f32
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
                        player_identifier.0.name.as_str(),
                        if player_identifier.1 == UwhColor::White {
                            265f32
                        } else {
                            1238f32
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
            let (x_off, text) =
                center_text_offset!(217f32, state.black.team_name, 45, self.assets.font);
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
            let (x_off, text) =
                center_text_offset!(220f32, state.white.team_name, 45, self.assets.font);
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
            // p1: Team White -> Team Black
            // p3: Team Black -> Referees
            // p5: Referees -> Fade out
            let p1 = ((state.white.members.len() + 3) / 4 * RPD_CARD_TIME) as f32;
            let p2 = p1 + ((state.black.members.len() + 3) / 4 * RPD_CARD_TIME) as f32;
            let p3 = p2 + ((state.referees.len() + 3) / 4 * RPD_CARD_TIME) as f32;

            let rpd_selector = match Instant::now()
                .duration_since(self.animation_register2)
                .as_f64() as f32
            {
                a if (0f32..=p1).contains(&a) => Some((
                    state.white.team_name.as_str(),
                    state.white.flag,
                    &state.white.members,
                    &self.assets.white_rpd,
                    BLACK,
                    (0f32, p1),
                )),
                a if (p1..=p2).contains(&a) => Some((
                    state.black.team_name.as_str(),
                    state.black.flag,
                    &state.black.members,
                    &self.assets.black_rpd,
                    WHITE,
                    (p1, p2),
                )),
                a if (p2..=p3).contains(&a) => Some((
                    "REFEREES",
                    None,
                    &state.referees,
                    &self.assets.red_rpd,
                    YELLOW,
                    (p2, p3),
                )),
                _ => None, // time after rpd display
            };

            if let Some((
                team_name,
                team_flag,
                card_repr,
                team_textures,
                text_color,
                (page_start, page_end),
            )) = rpd_selector
            {
                draw_texture_both!(team_textures.team_name_bg, 249f32, 32f32, WHITE);
                if let Some(flag) = team_flag {
                    let f_width = flag.width() * (170f32 / flag.height());
                    draw_texture_ex(
                        flag,
                        484f32 - f_width / 2f32,
                        57f32,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(f_width, 170f32)),
                            ..Default::default()
                        },
                    );
                    draw_rectangle(
                        484f32 - f_width / 2f32 + 1920f32,
                        57f32,
                        f_width,
                        170f32,
                        WHITE,
                    );
                }
                let (x_off, text) = center_text_offset!(
                    if team_flag.is_some() { 450f32 } else { 600f32 },
                    team_name,
                    100,
                    self.assets.font
                );
                draw_text_both_ex!(
                    text.as_str(),
                    if team_flag.is_some() { 680f32 } else { 361f32 } + x_off,
                    180f32,
                    TextParams {
                        font: self.assets.font,
                        font_size: 100,
                        color: text_color,
                        ..Default::default()
                    },
                    TextParams {
                        font: self.assets.font,
                        font_size: 100,
                        color: WHITE,
                        ..Default::default()
                    }
                );

                rpd_groups(card_repr)
                    .nth(
                        (Instant::now()
                            .duration_since(self.animation_register2)
                            .as_f64() as f32
                            - page_start) as usize
                            / RPD_CARD_TIME,
                    )
                    .map(|x| {
                        let no_cards = x.len() as f32;
                        const CARD_WIDTH: f32 = 440f32;
                        let margin = (1920f32 - CARD_WIDTH * no_cards) / (no_cards + 1f32);
                        x.iter().enumerate().for_each(
                            |(
                                i,
                                Member {
                                    name,
                                    role,
                                    number,
                                    geared_picture,
                                    picture,
                                },
                            )| {
                                draw_texture_both!(
                                    team_textures.frame_bg,
                                    i as f32 * (CARD_WIDTH + margin) + margin + 12f32,
                                    407f32,
                                    WHITE
                                );
                                draw_texture_both!(
                                    self.assets.frame_rpd,
                                    i as f32 * (CARD_WIDTH + margin) + margin,
                                    395f32,
                                    WHITE
                                );
                                draw_rectangle(
                                    i as f32 * (CARD_WIDTH + margin) + margin + 12f32 + 1920f32,
                                    407f32,
                                    416f32,
                                    416f32,
                                    WHITE,
                                );
                                let card_picture = if (Instant::now()
                                    .duration_since(self.animation_register2)
                                    .as_f64()
                                    as f32
                                    - page_start)
                                    > (page_end - page_start) / 2f32
                                {
                                    if geared_picture.is_some() {
                                        geared_picture
                                    } else {
                                        picture
                                    }
                                } else {
                                    picture
                                };
                                draw_texture_ex(
                                    card_picture.unwrap_or(self.assets.potrait_default.color),
                                    i as f32 * (CARD_WIDTH + margin) + margin + 12f32,
                                    407f32,
                                    WHITE,
                                    DrawTextureParams {
                                        dest_size: Some(vec2(416f32, 416f32)),
                                        ..Default::default()
                                    },
                                );
                                if let Some(number) = number {
                                    draw_texture_both!(
                                        self.assets.number_bg_rpd,
                                        i as f32 * (CARD_WIDTH + margin) + margin + 15f32,
                                        395f32,
                                        WHITE
                                    );
                                    let (x_off, text) = center_text_offset!(
                                        NUMBER_BG_WIDTH / 2f32,
                                        format!("#{}", number),
                                        40,
                                        self.assets.font
                                    );
                                    draw_text_both_ex!(
                                        text.as_str(),
                                        i as f32 * (CARD_WIDTH + margin) + margin + x_off + 25f32,
                                        445f32 as f32,
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 40,
                                            color: text_color,
                                            ..Default::default()
                                        },
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 40,
                                            color: WHITE,
                                            ..Default::default()
                                        }
                                    );
                                }
                                if let Some(role) = role {
                                    draw_texture_both!(
                                        team_textures.team_member_role_bg,
                                        i as f32 * (CARD_WIDTH + margin) + margin,
                                        340f32,
                                        WHITE
                                    );
                                    let (x_off, text) =
                                        center_text_offset!(200f32, role, 40, self.assets.font);
                                    draw_text_both_ex!(
                                        &text,
                                        i as f32 * (CARD_WIDTH + margin) + margin + 22f32 + x_off,
                                        377f32,
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 40,
                                            color: text_color,
                                            ..Default::default()
                                        },
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 40,
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
                                    i as f32 * (CARD_WIDTH + margin) + margin,
                                    847f32,
                                    WHITE
                                );
                                for (j, line) in lines.iter().take(3).enumerate() {
                                    let (x_off, text) =
                                        center_text_offset!(214f32, line, 33, self.assets.font);
                                    draw_text_both_ex!(
                                        text.as_str(),
                                        i as f32 * (CARD_WIDTH + margin) + margin + 4f32 + x_off,
                                        884f32 + 33f32 * j as f32,
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 33,
                                            color: text_color,
                                            ..Default::default()
                                        },
                                        TextParams {
                                            font: self.assets.font,
                                            font_size: 33,
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
