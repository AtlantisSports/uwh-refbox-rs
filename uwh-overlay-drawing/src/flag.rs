//! Handles everything to do with rendering flags like the GOAL flag
//! and penalty flags. Create an instance of `FlagRenderer` and push Flags into it.
//! Flags are discarded automatically after their 5 second show time as long as the draw function is called.

use macroquad::prelude::*;
use uwh_common::game_snapshot::GameSnapshot;

use crate::load_images::load;

/// Distance from the top of the screen from where the flags are rendered
const BASE_HEIGHT: f32 = 200f32;

/// Vertical space allocated to each flag
const FLAG_HEIGHT: f32 = 20f32;

pub enum FlagType {
    Goal,
    TD,
    BlackTimeout(u16),
    WhiteTimeout(u16),
}

struct Textures {
    black_goal: Texture2D,
    white_goal: Texture2D,
    white_penalty: Texture2D,
    black_penalty: Texture2D,
}

pub struct Flag {
    player_name: String,
    player_number: u16,
    flag_type: FlagType,
    animation_counter: u32,
}

impl Flag {
    pub fn new(player_name: String, player_number: u16, flag_type: FlagType) -> Self {
        Flag {
            player_name,
            player_number,
            flag_type,
            animation_counter: 0,
        }
    }
}

pub struct FlagRenderer {
    pub active_flags: Vec<Flag>,
    textures: Textures,
}

impl FlagRenderer {
    pub fn new() -> Self {
        Self {
            active_flags: Vec::new(),
            textures: Textures {
                black_goal: load!("../assets/alpha/1080/[PNG] 8K - Team Black Graphic.png"),
                white_goal: load!("../assets/color/1080/[PNG] 8K - Team White Graphic.png"),
                white_penalty: load!("../assets/color/1080/[PNG] 8K - Penalty White Graphic.png"),
                black_penalty: load!("../assets/color/1080/[PNG] 8K -  Penalty Black Graphic.png"),
            },
        }
    }

    pub fn draw(&mut self) {
        let b: f32 = 200f32;
        let f: f32 = 20f32;
        for (idx, flag) in self.active_flags.iter().enumerate() {
            // draw_texture(, x, y, color)
            draw_texture(
                self.textures.white_penalty,
                100f32,
                b + idx as f32 * f,
                WHITE,
            );
        }
    }
}
