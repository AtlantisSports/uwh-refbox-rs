use crate::{load_images::Textures, State};
use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;

mod final_scores;
mod in_game;
mod next_game;
mod overtime;
mod pre_game;
mod roster;

trait Interpolate {
    /// `value` must be a floater varying from 0 to 1, denoting the lowest to highest limits of the range
    fn interpolate_linear(&self, value: f32) -> f32;
}

impl Interpolate for (f32, f32) {
    fn interpolate_linear(&self, value: f32) -> f32 {
        (self.1 - self.0).mul_add(value, self.0)
    }
}

macro_rules! center_text_offset {
    ($field_width: expr, $string: expr, $font_size: literal, $font: expr) => {
        $field_width - measure_text($string, Some($font), $font_size, 1.0).width / 2f32
    };
}
pub(crate) use center_text_offset;

#[allow(dead_code)]
/// Utility function used to place overlay elements quickly through user input without recompiling
fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to init stdin");
    buffer.trim().parse().unwrap_or_default()
}

pub struct PageRenderer {
    pub animation_counter: f32,
    pub is_alpha_mode: bool,
    pub textures: Textures,
}
