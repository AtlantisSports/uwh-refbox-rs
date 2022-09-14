use crate::load_images::Textures;
use macroquad::prelude::*;

mod final_scores;
mod in_game;
mod next_game;
mod overtime_and_sudden_death;
mod pre_game;
mod roster;

pub(crate) trait Interpolate {
    /// `value` must be a floater varying from 0 to 1, denoting the lowest to highest limits of the range
    fn interpolate_linear(&self, value: f32) -> f32;
    fn interpolate_exponential_end(&self, value: f32) -> f32;
}

impl Interpolate for (f32, f32) {
    fn interpolate_linear(&self, value: f32) -> f32 {
        (self.1 - self.0).mul_add(value, self.0)
    }

    fn interpolate_exponential_end(&self, value: f32) -> f32 {
        let offset = ((self.1 - self.0).abs() + 1f32).powf(value);
        self.0
            + if self.0 > self.1 {
                -1f32 * offset
            } else {
                offset
            }
    }
}

macro_rules! center_text_offset {
    ($field_width: expr, $string: expr, $font_size: literal, $font: expr) => {
        $field_width - measure_text($string, Some($font), $font_size, 1.0).width / 2f32
    };
}
pub(crate) use center_text_offset;
use uwh_common::game_snapshot::TimeoutSnapshot;

#[allow(dead_code)]
/// Utility function used to place overlay elements quickly through user input without recompiling
pub fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {}: ", prompt);
    std::io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to init stdin");
    buffer.trim().parse().unwrap_or_default()
}

pub struct PageRenderer {
    /// Holds state for progression of an animation
    pub animation_counter: f32,
    /// Use if there are more than one simultenous animations
    pub secondary_animation_counter: f32,
    /// Should the alpha or color stream be rendered?
    pub is_alpha_mode: bool,
    /// Contains textures, alpha in alpha mode, color in color mode
    pub textures: Textures,
    /// We need to keep track of the last timeout snapshot in order to display information during the fade out
    pub last_timeout: TimeoutSnapshot,
}
