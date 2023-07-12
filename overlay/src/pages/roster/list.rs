use super::{draw_texture_both, fit_text, Interpolate, Justify, PageRenderer};
use crate::{
    pages::{draw_text_both, draw_text_both_ex, draw_texture_both_ex},
    State,
};
use coarsetime::Instant;
use macroquad::prelude::*;
use uwh_common::game_snapshot::Color as UwhColor;

const LIST_NUMBER_BG_WIDTH: f32 = 85f32;
const TEAM_BANNER_ROSTER_OFFSET: f32 = -650f32;

pub fn draw(renderer: &mut PageRenderer, state: &State) {
    let offset = if state.snapshot.secs_in_period == 181 {
        (0f32, TEAM_BANNER_ROSTER_OFFSET).interpolate_linear(
            Instant::now()
                .duration_since(renderer.animation_register1)
                .as_f64() as f32,
        )
    } else {
        if state.snapshot.secs_in_period != 169 {
            renderer.animation_register1 = Instant::now();
        }
        (0f32, TEAM_BANNER_ROSTER_OFFSET).interpolate_linear(1f32)
    };
    let timeout_alpha_offset = if state.snapshot.secs_in_period == 169 {
        renderer.animation_register2 = Instant::now();
        (1f32, 0f32).interpolate_linear(
            2f32 * (Instant::now()
                .duration_since(renderer.animation_register1)
                .as_f64() as f32),
        )
    } else {
        1f32
    };
    if let Some(logo) = renderer.assets.tournament_logo.as_ref() {
        if offset <= -210f32 {
            let x = (1920f32 - logo.color.width()) / 2f32;
            let y = 675f32 - logo.color.height();
            draw_texture_both!(
                logo,
                x,
                y,
                Color {
                    a: timeout_alpha_offset,
                    ..WHITE
                }
            );
        }
    }

    draw_texture_both!(
        renderer.assets.atlantis_logo,
        836f32,
        725f32,
        Color {
            a: timeout_alpha_offset,
            ..WHITE
        }
    );
    draw_texture_both!(
        renderer.assets.bottom,
        822f32,
        977f32,
        Color {
            a: timeout_alpha_offset,
            ..WHITE
        }
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
                    renderer.assets.team_white_banner,
                    if player_identifier.1 == UwhColor::White {
                        130f32
                    } else {
                        1108f32
                    },
                    60f32 * i as f32 + 220f32,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    }
                );
            } else {
                draw_texture_both!(
                    renderer.assets.team_black_banner,
                    if player_identifier.1 == UwhColor::White {
                        130f32
                    } else {
                        1108f32
                    },
                    60f32 * i as f32 + 220f32,
                    Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    }
                );
            }
            let (_, text) = fit_text(
                LIST_NUMBER_BG_WIDTH,
                &format!("#{}", player_identifier.0.number.unwrap()),
                35,
                renderer.assets.font,
                Justify::Left,
            );
            draw_text_ex(
                &text,
                if player_identifier.1 == UwhColor::White {
                    140f32
                } else {
                    1118f32
                },
                255f32 + 60f32 * i as f32,
                TextParams {
                    font: renderer.assets.font,
                    font_size: 35,
                    color: if player_identifier.1 == UwhColor::White {
                        Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        }
                    } else {
                        Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        }
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
                255f32 + 60f32 * i as f32,
                TextParams {
                    font: renderer.assets.font,
                    font_size: 35,
                    color: if player_identifier.1 == UwhColor::White {
                        Color {
                            a: timeout_alpha_offset,
                            ..BLACK
                        }
                    } else {
                        Color {
                            a: timeout_alpha_offset,
                            ..WHITE
                        }
                    },
                    ..Default::default()
                },
                TextParams {
                    font: renderer.assets.font,
                    font_size: 35,
                    color: Color {
                        a: timeout_alpha_offset,
                        ..WHITE
                    },
                    ..Default::default()
                }
            );
        }
    }
    draw_texture_both!(
        renderer.assets.team_information,
        130f32,
        710f32 + offset,
        Color {
            a: timeout_alpha_offset,
            ..WHITE
        }
    );
    let (x_off, text) = fit_text(
        434f32,
        &state.black.team_name,
        45,
        renderer.assets.font,
        Justify::Center,
    );
    draw_text_both!(
        text.as_str(),
        1350f32 + x_off,
        805f32 + offset,
        TextParams {
            font: renderer.assets.font,
            font_size: 45,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        }
    );
    let (x_off, text) = fit_text(
        440f32,
        &state.white.team_name,
        45,
        renderer.assets.font,
        Justify::Center,
    );
    draw_text_both_ex!(
        text.as_str(),
        135f32 + x_off,
        805f32 + offset,
        TextParams {
            font: renderer.assets.font,
            font_size: 45,
            color: Color {
                a: timeout_alpha_offset,
                ..BLACK
            },
            ..Default::default()
        },
        TextParams {
            font: renderer.assets.font,
            font_size: 45,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        }
    );
    if let Some(flag) = &state.white.flag {
        draw_texture_both_ex!(
            flag,
            580f32,
            738f32 + offset,
            Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            DrawTextureParams {
                dest_size: Some(vec2(180f32, 100f32)),
                ..Default::default()
            }
        );
    }
    if let Some(flag) = &state.black.flag {
        draw_texture_both_ex!(
            flag,
            1163f32,
            738f32 + offset,
            Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            DrawTextureParams {
                dest_size: Some(vec2(180f32, 100f32)),
                ..Default::default()
            }
        );
    }
    let (x_off, text) = fit_text(
        270f32,
        &format!("GAME #{}", &state.game_id.to_string()),
        25,
        renderer.assets.font,
        Justify::Center,
    );
    draw_text_ex(
        text.as_str(),
        830f32 + x_off,
        745f32 + offset,
        TextParams {
            font: renderer.assets.font,
            font_size: 25,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        },
    );
    let (x_off, text) = fit_text(
        248f32,
        &state.start_time,
        25,
        renderer.assets.font,
        Justify::Center,
    );
    draw_text_ex(
        text.as_str(),
        838f32 + x_off,
        780f32 + offset,
        TextParams {
            font: renderer.assets.font,
            font_size: 25,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        },
    );
    let (x_off, text) = fit_text(
        220f32,
        &state.pool,
        25,
        renderer.assets.font,
        Justify::Center,
    );
    draw_text_ex(
        text.as_str(),
        855f32 + x_off,
        815f32 + offset,
        TextParams {
            font: renderer.assets.font,
            font_size: 25,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        },
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
    let (x_off, text) = fit_text(180f32, &text, 50, renderer.assets.font, Justify::Center);
    draw_text_ex(
        text.as_str(),
        870f32 + x_off,
        1020f32,
        TextParams {
            font: renderer.assets.font,
            font_size: 50,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        },
    );
    draw_text_ex(
        "NEXT GAME",
        905f32,
        1044f32,
        TextParams {
            font: renderer.assets.font,
            font_size: 20,
            color: Color {
                a: timeout_alpha_offset,
                ..WHITE
            },
            ..Default::default()
        },
    );
}
