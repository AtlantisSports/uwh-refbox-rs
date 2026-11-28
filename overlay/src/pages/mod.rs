use crate::load_images::{Texture, Textures};
use coarsetime::Instant;
use macroquad::prelude::*;

mod final_scores;
mod in_game;
mod next_game;
mod overtime_and_sudden_death;
mod pre_game;
pub mod roster;

pub trait Interpolate {
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
        self.0 + if self.0 > self.1 { -offset } else { offset }
    }
}

/// Wrap the given `text` into lines that fit within the specified `width`.
///
/// Divides the `text` into multiple lines, breaking at whitespace such that new words go on a new
/// line if they overflow the `width`. Lines may still overflow if there is no whitespace to break
/// at, so use the `center_text_offset` macro to center lines and crop each one to fit the width.
pub fn multilinify(text: &str, width: f32, font: Option<&Font>, font_size: u16) -> Vec<String> {
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
    font: &Font,
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
        draw_texture(&$texture.color, $x, $y, $color);
        draw_texture(&$texture.alpha, $x + 1920f32, $y, $color);
    };
}
pub(crate) use draw_texture_both;

macro_rules! draw_texture_both_ex {
    ($texture: expr, $x: expr, $y: expr, $color: expr, $params: expr) => {
        draw_texture_ex(&$texture.color, $x, $y, $color, $params);
        draw_texture_ex(&$texture.alpha, $x + 1920f32, $y, $color, $params);
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

/// Where to start `text` so that it is centred on `centre`.
pub fn centred_x(font: &Font, text: &str, size: u16, centre: f32) -> f32 {
    centre - measure_text(text, Some(font), size, 1.0).width / 2.0
}

/// How much wider a box must be drawn for a word `text_width` px wide, when the box as drawn
/// has `room` px for it: nothing if the word fits, otherwise the overhang rounded up to a whole
/// even number, so that a box widened from its centre keeps its ends on whole pixels.
pub fn widen_for(text_width: f32, room: f32) -> f32 {
    if text_width <= room {
        0.0
    } else {
        ((text_width - room) / 2.0).ceil() * 2.0
    }
}

/// One strip of a piece of artwork: its columns `src_x..src_x + src_w`, drawn `dest_w` px wide
/// from `dest_x`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub src_x: f32,
    pub src_w: f32,
    pub dest_x: f32,
    pub dest_w: f32,
}

/// The strips that draw artwork `art_width` px wide from `x` with `extra` px added at column
/// `cut`: everything left of the cut and everything from it rightwards exactly as drawn, with
/// the single column at the cut repeated in between. The cut must be where the artwork is the
/// same from one column to the next, so the added part cannot be told from the rest and the
/// curved ends keep their shape.
pub fn widened_strips(art_width: f32, cut: f32, extra: f32, x: f32) -> [Strip; 3] {
    [
        Strip {
            src_x: 0.0,
            src_w: cut,
            dest_x: x,
            dest_w: cut,
        },
        Strip {
            src_x: cut,
            src_w: 1.0,
            dest_x: x + cut,
            dest_w: extra,
        },
        Strip {
            src_x: cut,
            src_w: art_width - cut,
            dest_x: x + cut + extra,
            dest_w: art_width - cut,
        },
    ]
}

/// `strip` without whatever lies left of `clip_x`, or `None` if nothing of it is left.
pub fn clip_left(strip: Strip, clip_x: f32) -> Option<Strip> {
    if strip.dest_w <= 0.0 {
        return None;
    }
    let hidden = clip_x - strip.dest_x;
    if hidden <= 0.0 {
        return Some(strip);
    }
    if hidden >= strip.dest_w {
        return None;
    }
    let hidden_share = hidden / strip.dest_w;
    Some(Strip {
        src_x: strip.src_x + strip.src_w * hidden_share,
        src_w: strip.src_w * (1.0 - hidden_share),
        dest_x: clip_x,
        dest_w: strip.dest_w - hidden,
    })
}

/// Draws `strips` of `tex` with their top at `y`, into both the colour picture and the
/// transparency picture.
pub fn draw_strips(tex: &Texture, strips: &[Strip], y: f32, color: Color) {
    let height = tex.color.height();
    for strip in strips {
        if strip.dest_w <= 0.0 {
            continue;
        }
        draw_texture_both_ex!(
            tex,
            strip.dest_x,
            y,
            color,
            DrawTextureParams {
                source: Some(Rect::new(strip.src_x, 0.0, strip.src_w, height)),
                dest_size: Some(vec2(strip.dest_w, height)),
                ..Default::default()
            }
        );
    }
}

/// Draws a box `extra` px wider than its artwork, widened from its centre: `x` is where the
/// artwork sits at its own width, and the wider box reaches `extra / 2` further each way. With
/// no `extra` the artwork is drawn exactly as it always was.
pub fn draw_widened(tex: &Texture, x: f32, y: f32, cut: f32, extra: f32, color: Color) {
    if extra <= 0.0 {
        draw_texture_both!(tex, x, y, color);
    } else {
        let strips = widened_strips(tex.color.width(), cut, extra, x - extra / 2.0);
        draw_strips(tex, &strips, y, color);
    }
}

/// A column in the plain middle of the time-and-period box, and one in the next-game box's,
/// where each can be widened (see `widened_strips`).
pub const TIME_BOX_CUT: f32 = 149.0;
pub const NEXT_GAME_BOX_CUT: f32 = 150.0;

/// How much wider the next-game box must be for this language's "NEXT GAME". The English
/// decides how much room the box has as drawn.
pub fn next_game_box_extra(font: &Font, text: &str) -> f32 {
    let width = |text| measure_text(text, Some(font), 20, 1.0).width;
    widen_for(width(text), width("NEXT GAME"))
}

/// A timeout or penalty-shot flag: the tab that slides out from behind the time box.
pub struct TimeoutFlag<'a> {
    pub font: &'a Font,
    /// The flag's two lines of text.
    pub lines: (&'a str, &'a str),
    /// The English lines, and where each starts once the flag is out. They decide where the
    /// text sits and how much room the flag has as drawn.
    pub english: (&'a str, &'a str),
    pub english_x: (f32, f32),
    /// How far the flag still has to slide out: 0 when fully out, down to -270 when hidden.
    pub slide: f32,
    /// How far right the flag sits because the time box in front of it is wider than its
    /// artwork (half of what the box gained).
    pub shift: f32,
}

impl TimeoutFlag<'_> {
    const SIZE: u16 = 20;
    /// Where the artwork's left end is once the flag is fully out, which is also the line it
    /// slides out from, hidden behind the time box.
    const X: f32 = 580.0;
    /// A column in the flag's plain body, left of a team flag's time, where it can be widened.
    const CUT: f32 = 140.0;
    /// The furthest the artwork slides.
    const TRAVEL: f32 = 270.0;

    fn width(&self, text: &str) -> f32 {
        measure_text(text, Some(self.font), Self::SIZE, 1.0).width
    }

    /// How much wider than its artwork the flag is, so that its longer line has the room the
    /// longer English line has. The flag is fixed behind the time box, so it grows rightwards.
    pub fn extra(&self) -> f32 {
        widen_for(
            self.width(self.lines.0).max(self.width(self.lines.1)),
            self.width(self.english.0).max(self.width(self.english.1)),
        )
    }

    /// The slide, lengthened so that a wider flag also starts fully hidden.
    fn slide(&self) -> f32 {
        self.slide * (Self::TRAVEL + self.extra()) / Self::TRAVEL
    }

    /// How far right of its usual place anything after the flag's text (a team flag's time) is.
    pub fn after_text(&self) -> f32 {
        self.slide() + self.shift + self.extra()
    }

    /// Draws the flag's artwork, `color` carrying its fade.
    pub fn draw_art(&self, tex: &Texture, color: Color) {
        let clip_x = Self::X + self.shift;
        let strips = widened_strips(
            tex.color.width(),
            Self::CUT,
            self.extra(),
            clip_x + self.slide(),
        );
        let shown: Vec<Strip> = strips
            .into_iter()
            .filter_map(|strip| clip_left(strip, clip_x))
            .collect();
        draw_strips(tex, &shown, 35.0, color);
    }

    /// Draws the flag's two lines, each centred where its English line is centred, moved right by
    /// half of what the flag gained. `color` (with its fade already applied) is the text's colour;
    /// the transparency half gets white with the same fade.
    pub fn draw_text(&self, color: Color) {
        let moved = self.slide() + self.shift + self.extra() / 2.0;
        let params = |color: Color| TextParams {
            font: Some(self.font),
            font_size: Self::SIZE,
            color,
            ..Default::default()
        };
        for (text, english, english_x, y) in [
            (self.lines.0, self.english.0, self.english_x.0, 67.0),
            (self.lines.1, self.english.1, self.english_x.1, 95.0),
        ] {
            let centre = english_x + self.width(english) / 2.0 + moved;
            draw_text_both_ex!(
                text,
                centre - self.width(text) / 2.0,
                y,
                params(color),
                params(Color {
                    a: color.a,
                    ..WHITE
                })
            );
        }
    }
}

pub struct PageRenderer {
    /// Holds state for progression of an animation
    pub animation_register1: Instant,
    /// Use if there are more than one simultenous animations
    pub animation_register2: Instant,
    pub animation_register0: Instant,
    pub animation_register3: bool,
    pub animation_register4: Instant,
    /// Contains textures, alpha in alpha mode, color in color mode
    pub assets: Textures,
    /// We need to keep track of the last timeout snapshot in order to display information during the fade out
    pub last_snapshot_timeout: Option<TimeoutSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_that_fits_does_not_widen_its_box() {
        assert_eq!(widen_for(103.8, 103.8), 0.0);
        assert_eq!(widen_for(60.0, 180.0), 0.0);
    }

    #[test]
    fn a_longer_word_widens_its_box_by_a_whole_even_amount() {
        // "PRÓXIMO PARTIDO" against "NEXT GAME".
        assert_eq!(widen_for(166.8, 103.8), 64.0);
        assert_eq!(widen_for(101.0, 100.0), 2.0);
        assert_eq!(widen_for(102.0, 100.0), 2.0);
        assert_eq!(widen_for(102.1, 100.0), 4.0);
    }

    #[test]
    fn a_widened_box_keeps_both_ends_as_drawn() {
        let [left, added, right] = widened_strips(298.0, 149.0, 40.0, 347.0);
        // The left end: the artwork's own columns, at their own width.
        assert_eq!((left.src_x, left.src_w), (0.0, 149.0));
        assert_eq!((left.dest_x, left.dest_w), (347.0, 149.0));
        // The added part repeats one plain column.
        assert_eq!((added.src_x, added.src_w), (149.0, 1.0));
        assert_eq!((added.dest_x, added.dest_w), (496.0, 40.0));
        // The right end: the rest of the artwork, at its own width, moved right by what was added.
        assert_eq!((right.src_x, right.src_w), (149.0, 149.0));
        assert_eq!((right.dest_x, right.dest_w), (536.0, 149.0));
    }

    #[test]
    fn a_strip_is_cut_off_at_the_line_it_slides_out_from() {
        let strip = Strip {
            src_x: 10.0,
            src_w: 100.0,
            dest_x: 500.0,
            dest_w: 100.0,
        };
        // Wholly right of the line: untouched.
        assert_eq!(clip_left(strip, 480.0), Some(strip));
        // Wholly left of it: nothing drawn.
        assert_eq!(clip_left(strip, 600.0), None);
        // Across it: only the part to the right, from the matching artwork columns.
        assert_eq!(
            clip_left(strip, 530.0),
            Some(Strip {
                src_x: 40.0,
                src_w: 70.0,
                dest_x: 530.0,
                dest_w: 70.0,
            })
        );
        // A repeated column that is cut keeps showing that same column.
        let repeated = Strip {
            src_x: 140.0,
            src_w: 1.0,
            dest_x: 500.0,
            dest_w: 60.0,
        };
        let cut = clip_left(repeated, 530.0).expect("half of it is right of the line");
        assert_eq!((cut.dest_x, cut.dest_w), (530.0, 30.0));
        assert!(cut.src_x >= 140.0 && cut.src_x + cut.src_w <= 141.0);
        // Nothing to draw where no width was added.
        assert_eq!(
            clip_left(
                Strip {
                    dest_w: 0.0,
                    ..repeated
                },
                0.0
            ),
            None
        );
    }
}
