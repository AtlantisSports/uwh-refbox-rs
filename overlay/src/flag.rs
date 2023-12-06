//! Handles everything to do with rendering flags like the GOAL flag
//! and penalty flags. Create an instance of `FlagRenderer` and push Flags into it.
//! Flags are discarded automatically after their 5 second show time as long as the draw function is called.

use crate::load_images::Texture;
use crate::pages::draw_text_both_ex;
use crate::pages::draw_texture_both;
use crate::pages::fit_text;
use crate::pages::Interpolate;
use crate::pages::Justify;
use macroquad::prelude::*;
use uwh_common::game_snapshot::Color as UWHColor;
use uwh_common::game_snapshot::PenaltyTime;

use crate::load_images::asset_load;

/// Distance from the top of the screen from where the flags are rendered
const BASE_HEIGHT: f32 = 150f32;

/// Vertical space allocated to each flag
const FLAG_HEIGHT: f32 = 70f32;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Type {
    Goal(UWHColor, bool),
    /// Third enum value is used to keep track of whether the flag was visited in last sync. Unvisited flags need to be deleted.
    Penalty(UWHColor, PenaltyTime, bool),
}

struct Textures {
    black_goal: Texture,
    white_goal: Texture,
    white_penalty: Texture,
    black_penalty: Texture,
    font: Font,
}

#[derive(Debug, Clone)]
pub struct Flag {
    player_name: String,
    player_number: u8,
    flag_type: Type,
    /// Index of the flag's position starting from the top flag.
    vertical_position: u32,
    alpha_animation_counter: f32,
    movement_animation_counter: f32,
}

impl Flag {
    pub const fn new(player_name: String, player_number: u8, flag_type: Type) -> Self {
        Self {
            player_name,
            player_number,
            flag_type,
            alpha_animation_counter: 0f32,
            vertical_position: 0,
            movement_animation_counter: 0f32,
        }
    }
}

pub struct Renderer {
    active_flags: Vec<Flag>,
    inactive_flags: Vec<Flag>,
    textures: Textures,
}

impl Renderer {
    pub fn add_flag(&mut self, mut flag: Flag, game_state: &crate::State) {
        flag.vertical_position = self.active_flags.len() as u32;
        flag.player_name = match flag.flag_type {
            Type::Penalty(UWHColor::Black, _, _) | Type::Goal(UWHColor::Black, _) => game_state
                .black
                .get_players()
                .find(|player| player.number.unwrap() == flag.player_number)
                .map(|player| player.name.clone())
                .unwrap_or_default(),
            _ => game_state
                .white
                .get_players()
                .find(|player| player.number.unwrap() == flag.player_number)
                .map(|player| player.name.clone())
                .unwrap_or_default(),
        };
        self.active_flags.push(flag);
    }

    pub fn reset(&mut self) {
        self.active_flags.clear();
        self.inactive_flags.clear();
    }

    pub fn new() -> Self {
        Self {
            active_flags: Vec::new(),
            inactive_flags: Vec::new(),
            textures: Textures {
                black_goal: asset_load!("Team Black.png"),
                white_goal: asset_load!("Team White.png"),
                white_penalty: asset_load!("Penalty White.png"),
                black_penalty: asset_load!("Penalty Black.png"),
                font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF"))
                    .unwrap(),
            },
        }
    }

    pub fn synchronize_flags(&mut self, game_state: &crate::State) {
        self.synchronize_goals(game_state);
        self.synchronize_penalties(UWHColor::Black, game_state);
        self.synchronize_penalties(UWHColor::White, game_state);
    }

    fn synchronize_goals(&mut self, game_state: &crate::State) {
        // mark all goal flags as unvisited
        for flag in &mut self.active_flags {
            if let Flag {
                flag_type: Type::Goal(_, is_visited),
                ..
            } = flag
            {
                *is_visited = false;
            }
        }

        if let Some(goal) = game_state.snapshot.recent_goal {
            // find the position of the flag in the local list, or else create new
            let flag_pos = self
                .active_flags
                .iter()
                .position(|flag| {
                    matches!(flag.flag_type, Type::Goal(color, _) if color == goal.0 )
                        && flag.player_number == goal.1
                })
                .unwrap_or_else(|| {
                    self.add_flag(
                        Flag::new(String::new(), goal.1, Type::Goal(goal.0, true)),
                        game_state,
                    );
                    self.active_flags.len() - 1
                });

            // get the goal flag in the snapshot and mark it as visited
            match self.active_flags.get_mut(flag_pos).unwrap() {
                Flag {
                    flag_type: Type::Goal(_, is_visited),
                    ..
                } => {
                    *is_visited = true;
                }
                _ => unreachable!(),
            }
        }
        // move out all deleted goal flags to be faded to the inactive flag section
        self.active_flags
            .iter()
            .filter_map(|x| {
                if let Type::Goal(_, false) = x.flag_type {
                    Some(x.clone())
                } else {
                    None
                }
            })
            .for_each(|x| self.inactive_flags.push(x));
        self.active_flags
            .retain(|x| !matches!(x.flag_type, Type::Goal(_, false)));
    }

    /// Used to synchronise penalty info from snapshot with the local penalty list.
    /// Local penalty flags marked unvisited by this function will be faded out and deleted by the draw function.
    fn synchronize_penalties(&mut self, team_color: UWHColor, game_state: &crate::State) {
        // mark all penalty flags as unvisited
        for flag in &mut self.active_flags {
            if let Flag {
                flag_type: Type::Penalty(color, _, is_visited),
                ..
            } = flag
            {
                if *color == team_color {
                    *is_visited = false;
                }
            }
        }

        // update or create local penalty flags for current team_color, marking each as visited if updated.
        for penalty in if team_color == UWHColor::Black {
            &game_state.snapshot.b_penalties
        } else {
            &game_state.snapshot.w_penalties
        } {
            if !matches!(penalty.time, PenaltyTime::Seconds(0)) {
                // find the penalty in the local list, create a new penalty if it doesn't exist.
                let flag_pos = self
                    .active_flags
                    .iter()
                    .position(|flag| {
                        matches!(flag.flag_type, Type::Penalty(color, _, _) if color == team_color )
                            && flag.player_number == penalty.player_number
                    })
                    .unwrap_or_else(|| {
                        self.add_flag(
                            Flag::new(
                                String::new(),
                                penalty.player_number,
                                Type::Penalty(team_color, penalty.time, true),
                            ),
                            game_state,
                        );
                        self.active_flags.len() - 1
                    });

                // update time on all the penalty flags
                match self.active_flags.get_mut(flag_pos).unwrap() {
                    Flag {
                        flag_type: Type::Penalty(_, time, is_visited),
                        ..
                    } => {
                        *time = penalty.time;
                        *is_visited = true;
                    }
                    _ => unreachable!(),
                }
            }
        }
        // move out all deleted flags to be faded to the inactive flag section
        self.active_flags
            .iter()
            .filter_map(|x| {
                if let Type::Penalty(color, _, false) = x.flag_type {
                    let mut y = x.clone();
                    if matches!(y.flag_type, Type::Penalty(_, PenaltyTime::Seconds(1), _)) {
                        y.flag_type = Type::Penalty(color, PenaltyTime::Seconds(0), false);
                    }
                    Some(y)
                } else {
                    None
                }
            })
            .for_each(|x| self.inactive_flags.push(x));
        self.active_flags
            .retain(|x| !matches!(x.flag_type, Type::Penalty(_, _, false)));
        // sort the flags based on: TD penalties on top, then time penalties sorted by longest time and lastly, goal callouts
        self.active_flags.sort_by(|a, b| {
            if let (
                // if both flags are penalty flags
                Flag {
                    flag_type: Type::Penalty(_, time_a, _),
                    ..
                },
                Flag {
                    flag_type: Type::Penalty(_, time_b, _),
                    ..
                },
            ) = (a, b)
            {
                if *time_a == PenaltyTime::TotalDismissal && *time_b == PenaltyTime::TotalDismissal
                // if they're both TD, keep same ordering
                {
                    std::cmp::Ordering::Equal
                } else if let (PenaltyTime::Seconds(time_a), PenaltyTime::Seconds(time_b)) =
                    // if they're both timed, sort by time remaining
                    (time_a, time_b)
                {
                    time_b.cmp(time_a)
                } else {
                    // if one is TD and the other timed, TD goes on top.
                    if *time_b == PenaltyTime::TotalDismissal {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                }
            } else if let (
                // if both flags are goal flags, keep same ordering
                Flag {
                    flag_type: Type::Goal(_, _),
                    ..
                },
                Flag {
                    flag_type: Type::Goal(_, _),
                    ..
                },
            ) = (a, b)
            {
                std::cmp::Ordering::Equal
            } else {
                // if one is a goal flag and the other a penalty flag, put the penalty flag on top
                if let Flag {
                    flag_type: Type::Goal(_, _),
                    ..
                } = a
                {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            }
        });
    }

    /// Responsible for drawing the flags, deleting them, etc.
    pub fn draw(&mut self) {
        for (idx, flag) in self.active_flags.iter_mut().enumerate() {
            if flag.alpha_animation_counter < 1f32 {
                flag.alpha_animation_counter += 1f32 / 60f32;
            }
            let alpha_offset = (0f32, 1f32).interpolate_linear(flag.alpha_animation_counter);
            let movement_offset = if flag.vertical_position == idx as u32 {
                0f32
            } else {
                if flag.movement_animation_counter > 1f32 {
                    flag.vertical_position = idx as u32;
                    flag.movement_animation_counter = 0f32;
                } else {
                    flag.movement_animation_counter += 1f32 / 60f32;
                }
                (
                    (flag.vertical_position as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT)
                        - (idx as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT),
                    0f32,
                )
                    .interpolate_linear(flag.movement_animation_counter)
            };
            let color = match flag.flag_type {
                Type::Goal(color, _) | Type::Penalty(color, _, _) => color,
            };
            let tex = match flag.flag_type {
                Type::Goal(_, _) => {
                    if color == UWHColor::White {
                        &self.textures.white_goal
                    } else {
                        &self.textures.black_goal
                    }
                }
                Type::Penalty(_, _, _) => {
                    if color == UWHColor::White {
                        &self.textures.white_penalty
                    } else {
                        &self.textures.black_penalty
                    }
                }
            };
            draw_texture_both!(
                tex,
                25f32,
                (idx as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + movement_offset,
                Color {
                    a: alpha_offset,
                    ..WHITE
                }
            );
            draw_text_both_ex!(
                format!("#{} {}", flag.player_number, flag.player_name).as_str(),
                160f32,
                (idx as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + movement_offset + 33f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 30,
                    color: if color == uwh_common::game_snapshot::Color::Black {
                        Color {
                            a: alpha_offset,
                            ..WHITE
                        }
                    } else {
                        Color {
                            a: alpha_offset,
                            ..BLACK
                        }
                    },
                    ..Default::default()
                },
                TextParams {
                    font: self.textures.font,
                    font_size: 30,
                    color: Color {
                        a: alpha_offset,
                        ..WHITE
                    },

                    ..Default::default()
                }
            );
            match flag.flag_type {
                Type::Goal(color, _) => draw_text_ex(
                    "GOAL",
                    45f32,
                    (idx as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + movement_offset + 33f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 30,
                        color: if color == uwh_common::game_snapshot::Color::Black {
                            Color {
                                a: alpha_offset,
                                ..WHITE
                            }
                        } else {
                            Color {
                                a: alpha_offset,
                                ..BLACK
                            }
                        },
                        ..Default::default()
                    },
                ),
                Type::Penalty(color, timeout, _) => {
                    let text = match timeout {
                        PenaltyTime::Seconds(s) => {
                            let mins = s / 60;
                            let secs = s % 60;

                            format!(
                                "{}:{}",
                                if mins < 10 {
                                    format!("0{mins}")
                                } else {
                                    format!("{mins}")
                                },
                                if secs < 10 {
                                    format!("0{secs}")
                                } else {
                                    format!("{secs}")
                                }
                            )
                        }
                        PenaltyTime::TotalDismissal => String::from("TD"),
                    };
                    let (x_off, text) =
                        fit_text(94f32, &text, 30, self.textures.font, Justify::Center);
                    draw_text_ex(
                        text.as_str(),
                        35f32 + x_off,
                        (idx as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + movement_offset + 33f32,
                        TextParams {
                            font: self.textures.font,
                            font_size: 30,
                            color: if color == uwh_common::game_snapshot::Color::Black {
                                Color {
                                    a: alpha_offset,
                                    ..WHITE
                                }
                            } else {
                                Color {
                                    a: alpha_offset,
                                    ..BLACK
                                }
                            },
                            ..Default::default()
                        },
                    );
                }
            }
        }
        for flag in &mut self.inactive_flags {
            let color = match flag.flag_type {
                Type::Goal(color, _) | Type::Penalty(color, _, _) => color,
            };
            flag.alpha_animation_counter -= 1f32 / 60f32;
            let alpha_offset = (0f32, 1f32).interpolate_linear(flag.alpha_animation_counter);
            let tex = match flag.flag_type {
                Type::Goal(_, _) => {
                    if color == UWHColor::White {
                        &self.textures.white_goal
                    } else {
                        &self.textures.black_goal
                    }
                }
                Type::Penalty(_, _, _) => {
                    if color == UWHColor::White {
                        &self.textures.white_penalty
                    } else {
                        &self.textures.black_penalty
                    }
                }
            };
            draw_texture_both!(
                tex,
                25f32,
                (flag.vertical_position as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT),
                Color {
                    a: alpha_offset,
                    ..WHITE
                }
            );
            draw_text_both_ex!(
                format!("#{} {}", flag.player_number, flag.player_name).as_str(),
                160f32,
                (flag.vertical_position as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + 33f32,
                TextParams {
                    font: self.textures.font,
                    font_size: 30,
                    color: if color == uwh_common::game_snapshot::Color::Black {
                        Color {
                            a: alpha_offset,
                            ..WHITE
                        }
                    } else {
                        Color {
                            a: alpha_offset,
                            ..BLACK
                        }
                    },
                    ..Default::default()
                },
                TextParams {
                    font: self.textures.font,
                    font_size: 30,
                    color: Color {
                        a: alpha_offset,
                        ..WHITE
                    },
                    ..Default::default()
                }
            );
            match flag.flag_type {
                Type::Goal(_, _) => draw_text_ex(
                    "GOAL",
                    45f32,
                    (flag.vertical_position as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + 33f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 30,
                        color: if color == uwh_common::game_snapshot::Color::Black {
                            Color {
                                a: alpha_offset,
                                ..WHITE
                            }
                        } else {
                            Color {
                                a: alpha_offset,
                                ..BLACK
                            }
                        },
                        ..Default::default()
                    },
                ),
                Type::Penalty(_, timeout, _) => {
                    let text = match timeout {
                        PenaltyTime::Seconds(s) => {
                            let mins = s / 60;
                            let secs = s % 60;

                            format!(
                                "{}:{}",
                                if mins < 10 {
                                    format!("0{mins}")
                                } else {
                                    format!("{mins}")
                                },
                                if secs < 10 {
                                    format!("0{secs}")
                                } else {
                                    format!("{secs}")
                                }
                            )
                        }
                        PenaltyTime::TotalDismissal => String::from("TD"),
                    };
                    let (x_off, text) =
                        fit_text(94f32, &text, 30, self.textures.font, Justify::Center);
                    draw_text_ex(
                        text.as_str(),
                        35f32 + x_off,
                        (flag.vertical_position as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + 33f32,
                        TextParams {
                            font: self.textures.font,
                            font_size: 30,
                            color: if color == uwh_common::game_snapshot::Color::Black {
                                Color {
                                    a: alpha_offset,
                                    ..WHITE
                                }
                            } else {
                                Color {
                                    a: alpha_offset,
                                    ..BLACK
                                }
                            },
                            ..Default::default()
                        },
                    );
                }
            }
        }
        // delete flags marked as unvisited and that have their alpha_animation_counter below zero (finihed fade out)
        self.inactive_flags
            .retain(|x| x.alpha_animation_counter > 0f32);
    }
}
