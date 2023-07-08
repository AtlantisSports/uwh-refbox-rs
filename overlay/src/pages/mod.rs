use crate::load_images::Textures;
use coarsetime::Instant;
use macroquad::prelude::*;

mod final_scores;
mod in_game;
mod next_game;
mod overtime_and_sudden_death;
mod pre_game;
pub mod roster;

pub(crate) trait Interpolate {
    /// `value` must be a `float` normally varying from `0f32` to `1f32`
    fn interpolate_linear(&self, value: f32) -> f32;
    /// Performs exponential interpolation towards the end of the range.
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

/// Wrap the given `text` into lines that fit within the specified `width`.
///
/// Divides the `text` into multiple lines, breaking at whitespace such that new words go on a new
/// line if they overflow the `width`. Lines may still overflow if there is no whitespace to break
/// at, so use the `center_text_offset` macro to center lines and crop each one to fit the width.
pub fn multilinify(text: &str, width: f32, font: Option<Font>, font_size: u16) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let word_width = measure_text(word, font, font_size, 1.0).width;
        let line_width = measure_text(&current_line, font, font_size, 1.0).width;

        if line_width + word_width <= width {
            // Word fits within the current line
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        } else {
            // Word doesn't fit within the current line, start a new line
            lines.push(current_line.clone());
            current_line = word.to_string();
        }
    }

    // Push the remaining text as the last line
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

pub enum Justify {
    Left,
    Center,
    #[allow(dead_code)]
    Right,
}

/// Calculates the offset for fitting text within a text field.
///
/// The function takes the following parameters:
/// - `field_width`: Width of the text field.
/// - `text`: The string to be fitted within the text field.
/// - `font_size`: The font size.
/// - `font`: The font used for rendering the text.
///
/// Returns a tuple containing the offset from the left side of the text field
/// to render text from, and the modified string that fits within the field.
pub fn fit_text(
    field_width: f32,
    text: &str,
    font_size: u16,
    font: Font,
    justify: Justify,
) -> (f32, String) {
    let mut text = text.to_string();

    let mut popped = false;
    while field_width < measure_text(text.as_str(), Some(font), font_size, 1.0).width {
        text.pop();
        popped = true;
    }

    let text = if popped {
        if field_width < measure_text(&(text.clone() + ".."), Some(font), font_size, 1.0).width {
            while field_width
                < measure_text(&(text.clone() + ".."), Some(font), font_size, 1.0).width
            {
                text.pop();
            }
            text + ".."
        } else {
            text + ".."
        }
    } else {
        text
    };
    let x_off = match justify {
        Justify::Left => 0f32,
        Justify::Center => {
            (field_width / 2f32)
                - measure_text(text.as_str(), Some(font), font_size, 1.0).width / 2f32
        }
        Justify::Right => {
            field_width - measure_text(text.as_str(), Some(font), font_size, 1.0).width / 2f32
        }
    };
    (x_off, text)
}

macro_rules! draw_texture_both {
    ($texture: expr, $x: expr, $y: expr, $color: expr) => {
        draw_texture($texture.color, $x, $y, $color);
        draw_texture($texture.alpha, $x + 1920f32, $y, $color);
    };
}
pub(crate) use draw_texture_both;

macro_rules! draw_texture_both_ex {
    ($texture: expr, $x: expr, $y: expr, $color: expr, $params: expr) => {
        draw_texture_ex($texture.color, $x, $y, $color, $params);
        draw_texture_ex($texture.alpha, $x + 1920f32, $y, $color, $params);
    };
}
pub(crate) use draw_texture_both_ex;

macro_rules! draw_text_both {
    ($text: expr, $x: expr, $y: expr, $params: expr) => {
        draw_text_ex($text, $x, $y, $params);
        draw_text_ex($text, $x + 1920f32, $y, $params);
    };
}
pub(crate) use draw_text_both;

macro_rules! draw_text_both_ex {
    ($text: expr, $x: expr, $y: expr, $params_color: expr, $params_alpha: expr) => {
        draw_text_ex($text, $x, $y, $params_color);
        draw_text_ex($text, $x + 1920f32, $y, $params_alpha);
    };
}
pub(crate) use draw_text_both_ex;

use uwh_common::game_snapshot::TimeoutSnapshot;

#[allow(dead_code)]
/// Utility function used to place overlay elements quickly through user input without recompiling
pub fn get_input<T: std::str::FromStr + std::default::Default>(prompt: &str) -> T {
    let mut buffer = String::new();
    println!(" Enter {prompt}: ");
    std::io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to init stdin");
    buffer.trim().parse().unwrap_or_default()
}

pub struct PageRenderer {
    /// Holds state for progression of an animation
    pub animation_register1: Instant,
    /// Use if there are more than one simultenous animations
    pub animation_register2: Instant,
    pub animation_register0: Instant,
    pub animation_register3: bool,
    /// Contains textures, alpha in alpha mode, color in color mode
    pub assets: Textures,
    pub bg: Texture2D,
    /// We need to keep track of the last timeout snapshot in order to display information during the fade out
    pub last_snapshot_timeout: TimeoutSnapshot,
}
