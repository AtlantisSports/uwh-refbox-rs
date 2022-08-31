//! Handles everything to do with rendering flags like the GOAL flag
//! and penalty flags. Create an instance of `FlagRenderer` and push Flags into it.
//! Flags are discarded automatically after their 5 second show time as long as the draw function is called.

use crate::pages::center_text_offset;
use crate::pages::Interpolate;
use macroquad::prelude::*;
use uwh_common::game_snapshot::Color as UWHColor;
use uwh_common::game_snapshot::PenaltyTime;

use crate::load_images::load;

/// Distance from the top of the screen from where the flags are rendered
const BASE_HEIGHT: f32 = 150f32;

/// Vertical space allocated to each flag
const FLAG_HEIGHT: f32 = 70f32;

#[derive(PartialEq, Debug)]
pub enum FlagType {
    Goal(UWHColor),
    /// Third enum value is used to keep track of whether the flag was visited in last sync. Unvisited flags need to be deleted.
    Penalty(UWHColor, PenaltyTime, bool),
}

struct Textures {
    black_goal: Texture2D,
    white_goal: Texture2D,
    white_penalty: Texture2D,
    black_penalty: Texture2D,
    font: Font,
}

#[derive(Debug)]
pub struct Flag {
    player_name: String,
    player_number: u8,
    flag_type: FlagType,
    /// Index of the flag's position starting from the top flag.
    vertical_position: u32,
    alpha_animation_counter: f32,
    movement_animation_counter: f32,
}

impl Flag {
    pub fn new(player_name: String, player_number: u8, flag_type: FlagType) -> Self {
        Flag {
            player_name,
            player_number,
            flag_type,
            alpha_animation_counter: 0f32,
            vertical_position: 0,
            movement_animation_counter: 0f32,
        }
    }
}

pub struct FlagRenderer {
    active_flags: Vec<Flag>,
    textures: Textures,
    is_alpha_mode: bool,
}

impl FlagRenderer {
    pub fn add_penalty_flag(&mut self, mut flag: Flag, game_state: &crate::State) {
        flag.vertical_position = self.active_flags.len() as u32;
        flag.player_name = match flag.flag_type {
            FlagType::Penalty(UWHColor::Black, _, _) => game_state
                .black
                .players
                .iter()
                .find(|player| player.1 == flag.player_number)
                .map(|player| player.0.clone())
                .unwrap_or(String::from("Unknown Player")),
            _ => game_state
                .white
                .players
                .iter()
                .find(|player| player.1 == flag.player_number)
                .map(|player| player.0.clone())
                .unwrap_or(String::from("Unknown Player")),
        };
        self.active_flags.push(flag);
    }

    pub fn new(is_alpha_mode: bool) -> Self {
        Self {
            active_flags: Vec::new(),
            is_alpha_mode,
            textures: if is_alpha_mode {
                Textures {
                    black_goal: load!("../assets/alpha/1080/[PNG] 8K - Team Black Graphic.png"),
                    white_goal: load!("../assets/alpha/1080/[PNG] 8K - Team White Graphic.png"),
                    white_penalty: load!("../assets/alpha/1080/Penalty White Graphic.png"),
                    black_penalty: load!("../assets/alpha/1080/Penalty Black Graphic.png"),
                    font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF"))
                        .unwrap(),
                }
            } else {
                Textures {
                    black_goal: load!("../assets/color/1080/[PNG] 8K - Team Black Graphic.png"),
                    white_goal: load!("../assets/color/1080/[PNG] 8K - Team White Graphic.png"),
                    white_penalty: load!("../assets/color/1080/Penalty White Graphic.png"),
                    black_penalty: load!("../assets/color/1080/Penalty Black Graphic.png"),
                    font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF"))
                        .unwrap(),
                }
            },
        }
    }

    /// Used to synchronise penalty info from snapshot with the local penalty list.
    /// Local penalty flags marked unvisited by this function will be faded out and deleted by the draw function.
    pub fn synchronize_penalties(&mut self, team_color: UWHColor, game_state: &crate::State) {
        // mark all penalty flags as unvisited
        for flag in &mut self.active_flags {
            if let Flag {
                flag_type: FlagType::Penalty(color, _, is_visited),
                ..
            } = flag
            {
                if *color == team_color {
                    *is_visited = false
                }
            }
        }

        // update or create local penalty flags for current team_color, marking each as visited if updated.
        for penalty in if team_color == UWHColor::Black {
            &game_state.snapshot.b_penalties
        } else {
            &game_state.snapshot.w_penalties
        } {
            // let penalty: PenaltySnapshot = penalty;
            // find the penalty in the local list, create a new penalty if it doesn't exist.
            let flag_pos = self
                .active_flags
                .iter()
                .position(|flag| {
                    matches!(flag.flag_type, FlagType::Penalty(color, _, _) if color == team_color )
                        && flag.player_number == penalty.player_number
                })
                .unwrap_or_else(|| {
                    self.add_penalty_flag(
                        Flag::new(
                            String::new(),
                            penalty.player_number,
                            FlagType::Penalty(team_color, penalty.time, true),
                        ),
                        game_state,
                    );
                    self.active_flags.len() - 1
                });

            // update time on all the penalty flags
            match self.active_flags.get_mut(flag_pos).unwrap() {
                Flag {
                    flag_type: FlagType::Penalty(_, time, is_visited),
                    ..
                } => {
                    *time = penalty.time;
                    *is_visited = true;
                }
                _ => unreachable!(),
            }
        }

        // sort the flags based on: TD penalties on top, then time penalties sorted by longest time and lastly, goal callouts
        self.active_flags.sort_by(|a, b| {
            if let (
                // if both flags are penalty flags
                Flag {
                    flag_type: FlagType::Penalty(_, time_a, _),
                    ..
                },
                Flag {
                    flag_type: FlagType::Penalty(_, time_b, _),
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
                    flag_type: FlagType::Goal(_),
                    ..
                },
                Flag {
                    flag_type: FlagType::Goal(_),
                    ..
                },
            ) = (a, b)
            {
                std::cmp::Ordering::Equal
            } else {
                // if one is a goal flag and the other a penalty flag, put the penalty flag on top
                if let Flag {
                    flag_type: FlagType::Goal(_),
                    ..
                } = a
                {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            }
        })
    }

    /// Responsible for drawing the flags, deleting them, etc.
    pub fn draw(&mut self) {
        for (idx, flag) in self.active_flags.iter_mut().enumerate() {
            let alpha_offset = if let FlagType::Goal(_) = flag.flag_type {
                flag.alpha_animation_counter += 1f32 / (60f32 * 5f32);
                // fade in goal flag, start fade out at the fourth second.
                if flag.alpha_animation_counter < 0.2f32 {
                    (0f32, 255f32).interpolate_linear(flag.alpha_animation_counter * 5f32)
                } else if flag.alpha_animation_counter > 0.8f32 {
                    (255f32, 0f32).interpolate_linear(flag.alpha_animation_counter * 5f32 - 4f32)
                } else {
                    255f32
                }
            } else {
                // fade in the flag, but fade it out when it is marked unvisited by the syncronize function
                if let FlagType::Penalty(_, _, false) = flag.flag_type {
                    flag.alpha_animation_counter -= 1f32 / 60f32;
                } else if flag.alpha_animation_counter < 1f32 {
                    flag.alpha_animation_counter += 1f32 / 60f32;
                }
                (0f32, 255f32).interpolate_linear(flag.alpha_animation_counter)
            };
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
                    (BASE_HEIGHT + flag.vertical_position as f32 * FLAG_HEIGHT)
                        - (BASE_HEIGHT + idx as f32 * FLAG_HEIGHT),
                    0f32,
                )
                    .interpolate_linear(flag.movement_animation_counter)
            };
            draw_texture(
                match flag.flag_type {
                    FlagType::Goal(color) => {
                        if color == UWHColor::White {
                            self.textures.white_goal
                        } else {
                            self.textures.black_goal
                        }
                    }
                    FlagType::Penalty(color, _, _) => {
                        if color == UWHColor::White {
                            self.textures.white_penalty
                        } else {
                            self.textures.black_penalty
                        }
                    }
                },
                25f32,
                BASE_HEIGHT + idx as f32 * FLAG_HEIGHT + movement_offset,
                Color::from_rgba(255, 255, 255, alpha_offset as u8),
            );
            if !self.is_alpha_mode {
                draw_text_ex(
                    format!("#{} {}", flag.player_number, flag.player_name).as_str(),
                    160f32,
                    BASE_HEIGHT + idx as f32 * FLAG_HEIGHT + movement_offset + 33f32,
                    TextParams {
                        font: self.textures.font,
                        font_size: 30,
                        color: Color::from_rgba(255, 255, 255, alpha_offset as u8),
                        ..Default::default()
                    },
                );
                match flag.flag_type {
                    FlagType::Goal(_) => draw_text_ex(
                        "GOAL",
                        45f32,
                        BASE_HEIGHT + idx as f32 * FLAG_HEIGHT + movement_offset + 33f32,
                        TextParams {
                            font: self.textures.font,
                            font_size: 30,
                            color: Color::from_rgba(255, 255, 255, alpha_offset as u8),
                            ..Default::default()
                        },
                    ),
                    FlagType::Penalty(_, timeout, _) => {
                        let text = &match timeout {
                            PenaltyTime::Seconds(s) => {
                                let mins = s / 60;
                                let secs = s % 60;

                                format!("{}:{}", mins, secs)
                            }
                            PenaltyTime::TotalDismissal => String::from("TD"),
                        };
                        let x_off = center_text_offset!(47f32, text, 30, self.textures.font);
                        draw_text_ex(
                            text,
                            35f32 + x_off,
                            BASE_HEIGHT + idx as f32 * FLAG_HEIGHT + movement_offset + 33f32,
                            TextParams {
                                font: self.textures.font,
                                font_size: 30,
                                color: Color::from_rgba(255, 255, 255, alpha_offset as u8),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
        // delete GOAL flags that have been around for more than five seconds
        self.active_flags.retain(|x| {
            !(x.alpha_animation_counter > 1f32 && matches!(x.flag_type, FlagType::Goal(_)))
        });
        // delete penalty flags marked as unvisited and that have their alpha_animation_counter below zero (finihed fade out)
        self.active_flags.retain(|x| {
            !(x.alpha_animation_counter < 0f32
                && matches!(x.flag_type, FlagType::Penalty(_, _, false)))
        });
    }
}
