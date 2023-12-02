use super::{draw_texture_both, fit_text, Justify, PageRenderer};
use crate::{
    pages::{draw_text_both_ex, draw_texture_both_ex, multilinify},
    Member, State,
};
use coarsetime::Instant;
use macroquad::prelude::*;

const RPD_GROUP_TIME: f32 = 10f32;
const RPD_NUMBER_BG_WIDTH: f32 = 100f32;
const CARD_WIDTH: f32 = 440f32;

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

pub fn get_cycle_times(state: &State) -> (f32, f32, f32) {
    // Page Transition Points:
    // p1: Team White -> Team Black
    // p2: Team Black -> Referees
    // p3: Referees -> Fade out
    let p1 = if !state.white.members.is_empty()
        && state
            .white
            .members
            .iter()
            .any(|member| member.geared_picture.is_some() || member.picture.is_some())
    {
        4.5 + ((state.white.members.len() + 3) / 4 * RPD_GROUP_TIME as usize) as f32
    } else {
        0f32
    };
    let p2 = p1
        + if !state.black.members.is_empty()
            && state
                .black
                .members
                .iter()
                .any(|member| member.geared_picture.is_some() || member.picture.is_some())
        {
            4.5 + ((state.black.members.len() + 3) / 4 * RPD_GROUP_TIME as usize) as f32
        } else {
            0f32
        };
    let p3 = p2
        + if !state.referees.is_empty()
            && state
                .referees
                .iter()
                .any(|referee| referee.geared_picture.is_some() || referee.picture.is_some())
        {
            4.5 + ((state.referees.len() + 3) / 4 * RPD_GROUP_TIME as usize) as f32
        } else {
            0f32
        };
    (p1, p2, p3)
}

pub fn draw(renderer: &mut PageRenderer, state: &State) {
    let (p1, p2, p3) = get_cycle_times(state);
    let (team_name, team_flag, card_repr, team_textures, text_color, (page_start, page_end)) =
        match Instant::now()
            .duration_since(renderer.animation_register2)
            .as_f64() as f32
        {
            a if (0f32..=p1).contains(&a) => (
                state.white.team_name.as_str(),
                &state.white.flag,
                &state.white.members,
                &renderer.assets.white_rpd,
                BLACK,
                (0f32, p1),
            ),
            a if (p1..=p2).contains(&a) => (
                state.black.team_name.as_str(),
                &state.black.flag,
                &state.black.members,
                &renderer.assets.black_rpd,
                WHITE,
                (p1, p2),
            ),
            a if (p2..=p3).contains(&a) => {
                renderer.animation_register0 = Instant::now();
                (
                    "REFEREES",
                    &None,
                    &state.referees,
                    &renderer.assets.red_rpd,
                    YELLOW,
                    (p2, p3),
                )
            }
            _ => {
                renderer.pre_game_display(state);
                return;
            } // time after rpd display
        };

    let since_page_start = Instant::now()
        .duration_since(renderer.animation_register2)
        .as_f64() as f32
        - page_start;
    let to_page_end = page_end
        - Instant::now()
            .duration_since(renderer.animation_register2)
            .as_f64() as f32;
    let top_banner_alpha = if to_page_end <= 1.5 {
        2f32.mul_add(to_page_end, -2f32)
    } else {
        2f32 * (since_page_start - 1.5f32)
    };
    draw_texture_both!(
        team_textures.team_name_bg,
        249f32,
        32f32,
        Color {
            a: top_banner_alpha,
            ..WHITE
        }
    );
    if let Some(flag) = team_flag {
        let f_width = flag.color.width() * (170f32 / flag.color.height());
        draw_texture_both_ex!(
            flag,
            484f32 - f_width / 2f32,
            57f32,
            Color {
                a: top_banner_alpha,
                ..WHITE
            },
            DrawTextureParams {
                dest_size: Some(vec2(f_width, 170f32)),
                ..Default::default()
            }
        );
    }
    let (x_off, text) = fit_text(
        if team_flag.is_some() { 900f32 } else { 1200f32 },
        team_name,
        100,
        renderer.assets.font,
        Justify::Center,
    );
    draw_text_both_ex!(
        text.as_str(),
        if team_flag.is_some() { 680f32 } else { 361f32 } + x_off,
        180f32,
        TextParams {
            font: renderer.assets.font,
            font_size: 100,
            color: Color {
                a: top_banner_alpha,
                ..text_color
            },
            ..Default::default()
        },
        TextParams {
            font: renderer.assets.font,
            font_size: 100,
            color: Color {
                a: top_banner_alpha,
                ..WHITE
            },
            ..Default::default()
        }
    );
    if let Some(cards) =
        rpd_groups(card_repr).nth((since_page_start - 3f32).div_euclid(RPD_GROUP_TIME) as usize)
    {
        let since_rpdgroup_start = (since_page_start - 3f32)
            .div_euclid(RPD_GROUP_TIME)
            .mul_add(-RPD_GROUP_TIME, since_page_start)
            - 3f32;
        let to_rpdgroup_end = RPD_GROUP_TIME - since_rpdgroup_start;
        let rpdgroup_alpha = if since_page_start <= 3f32 {
            0f32
        } else if to_rpdgroup_end <= 0.5 {
            2f32 * to_rpdgroup_end
        } else {
            2f32 * since_rpdgroup_start
        };
        let rpd_picture_alpha = (RPD_GROUP_TIME / 2f32 - since_rpdgroup_start) * 2f32;
        let no_cards = cards.len() as f32;
        let margin = CARD_WIDTH.mul_add(-no_cards, 1920f32) / (no_cards + 1f32);
        cards.iter().enumerate().for_each(
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
                    (i as f32).mul_add(CARD_WIDTH + margin, margin) + 12f32,
                    407f32,
                    Color {
                        a: rpdgroup_alpha,
                        ..WHITE
                    }
                );
                draw_texture_both!(
                    renderer.assets.frame_rpd,
                    (i as f32).mul_add(CARD_WIDTH + margin, margin),
                    395f32,
                    Color {
                        a: rpdgroup_alpha,
                        ..WHITE
                    }
                );
                if let (Some(picture), Some(geared_picture)) = (picture, geared_picture) {
                    draw_texture_both_ex!(
                        *picture,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin) + 12f32,
                        407f32,
                        Color {
                            a: rpdgroup_alpha.min(rpd_picture_alpha),
                            ..WHITE
                        },
                        DrawTextureParams {
                            dest_size: Some(vec2(416f32, 416f32)),
                            ..Default::default()
                        }
                    );
                    draw_texture_both_ex!(
                        *geared_picture,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin) + 12f32,
                        407f32,
                        Color {
                            a: rpdgroup_alpha.min(-rpd_picture_alpha),
                            ..WHITE
                        },
                        DrawTextureParams {
                            dest_size: Some(vec2(416f32, 416f32)),
                            ..Default::default()
                        }
                    );
                } else if let Some(picture) = picture {
                    draw_texture_both_ex!(
                        *picture,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin) + 12f32,
                        407f32,
                        Color {
                            a: rpdgroup_alpha,
                            ..WHITE
                        },
                        DrawTextureParams {
                            dest_size: Some(vec2(416f32, 416f32)),
                            ..Default::default()
                        }
                    );
                } else if let Some(picture) = geared_picture {
                    draw_texture_both_ex!(
                        *picture,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin) + 12f32,
                        407f32,
                        Color {
                            a: rpdgroup_alpha,
                            ..WHITE
                        },
                        DrawTextureParams {
                            dest_size: Some(vec2(416f32, 416f32)),
                            ..Default::default()
                        }
                    );
                }
                if let Some(number) = number {
                    draw_texture_both!(
                        renderer.assets.number_bg_rpd,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin),
                        395f32,
                        Color {
                            a: rpdgroup_alpha,
                            ..WHITE
                        }
                    );
                    let (x_off, text) = fit_text(
                        RPD_NUMBER_BG_WIDTH,
                        &format!("#{number}"),
                        40,
                        renderer.assets.font,
                        Justify::Left,
                    );
                    draw_text_ex(
                        text.as_str(),
                        (i as f32).mul_add(CARD_WIDTH + margin, margin) + x_off + 15f32,
                        445f32,
                        TextParams {
                            font: renderer.assets.font,
                            font_size: 40,
                            color: Color {
                                a: rpdgroup_alpha,
                                ..text_color
                            },
                            ..Default::default()
                        },
                    );
                }
                if let Some(role) = role {
                    draw_texture_both!(
                        team_textures.team_member_role_bg,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin),
                        340f32,
                        Color {
                            a: rpdgroup_alpha,
                            ..WHITE
                        }
                    );
                    let (x_off, text) =
                        fit_text(400f32, role, 40, renderer.assets.font, Justify::Center);
                    draw_text_both_ex!(
                        &text,
                        (i as f32).mul_add(CARD_WIDTH + margin, margin) + 22f32 + x_off,
                        377f32,
                        TextParams {
                            font: renderer.assets.font,
                            font_size: 40,
                            color: Color {
                                a: rpdgroup_alpha,
                                ..text_color
                            },
                            ..Default::default()
                        },
                        TextParams {
                            font: renderer.assets.font,
                            font_size: 40,
                            color: Color {
                                a: rpdgroup_alpha,
                                ..WHITE
                            },
                            ..Default::default()
                        }
                    );
                }

                let lines = multilinify(name, 400f32, Some(renderer.assets.font), 33);
                let text_box_texture = match lines.len() {
                    1 => &team_textures.single_line_name_bg,
                    2 => &team_textures.double_line_name_bg,
                    _ => &team_textures.triple_line_name_bg,
                };
                draw_texture_both!(
                    text_box_texture,
                    (i as f32).mul_add(CARD_WIDTH + margin, margin),
                    847f32,
                    Color {
                        a: rpdgroup_alpha,
                        ..WHITE
                    }
                );
                let text_height = measure_text("Q", Some(renderer.assets.font), 33, 1.0).height;
                let v_margin = text_height.mul_add(
                    -(lines.len().min(3) as f32),
                    text_box_texture.color.height(),
                ) / (lines.len().min(3) as f32 + 1f32);
                for (j, line) in lines.iter().take(3).enumerate() {
                    let (x_off, text) =
                        fit_text(400f32, line, 33, renderer.assets.font, Justify::Center);
                    draw_text_both_ex!(
                        text.as_str(),
                        (CARD_WIDTH + margin).mul_add(i as f32, margin + 12f32 + x_off),
                        (v_margin + text_height).mul_add(j as f32 + 1f32, 847f32),
                        TextParams {
                            font: renderer.assets.font,
                            font_size: 33,
                            color: Color {
                                a: rpdgroup_alpha,
                                ..text_color
                            },
                            ..Default::default()
                        },
                        TextParams {
                            font: renderer.assets.font,
                            font_size: 33,
                            color: Color {
                                a: rpdgroup_alpha,
                                ..WHITE
                            },
                            ..Default::default()
                        }
                    );
                }
            },
        );
    }
}
