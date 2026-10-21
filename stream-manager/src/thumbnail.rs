//! Draws each game's YouTube thumbnail in the "action" style (Black vs White): the event's
//! portal banner in the background, the dark-cap team on a black half and the light-cap team on
//! a white half, split by a slanted Atlantis teal line, with the game details along the bottom.
//!
//! The look is set by the constants below. Change [`DESIGN_VERSION`] whenever the look changes,
//! so that the next Prepare uploads the new design to videos that already have a thumbnail.

use crate::BoxError;
use image::{
    DynamicImage, ExtendedColorType, GrayImage, RgbaImage,
    codecs::jpeg::JpegEncoder,
    imageops::{self, FilterType},
};
use serde::{Deserialize, Serialize};
use tiny_skia::{
    Color, ColorU8, FillRule, GradientStop, LinearGradient, Mask, MaskType, Paint, Path,
    PathBuilder, Pixmap, PixmapPaint, Point, Rect, Shader, SpreadMode, Stroke, Transform,
};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

/// Bump this whenever the look changes.
pub const DESIGN_VERSION: u32 = 5;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const CENTER_X: f32 = WIDTH as f32 / 2.0;
/// YouTube's size limit for a thumbnail file.
const MAX_BYTES: usize = 2 * 1024 * 1024;

const FONT: &[u8] = include_bytes!("../../overlay/assets/BAHNSCHRIFT.TTF");
/// The overlay's white Atlantis Sports logo, without the "TM" mark.
const BRAND_LOGO: &[u8] = include_bytes!("../assets/Atlantis Logo.png");

// ---- Design ----

type Rgb = [u8; 3];
const BLACK: Rgb = [0, 0, 0];
const WHITE: Rgb = [255, 255, 255];
const INK: Rgb = [0x07, 0x09, 0x0c];
const PAPER: Rgb = [0xf7, 0xf9, 0xfb];
/// Atlantis teal, from the light to the deep end of the logo's "A".
const TEAL_LIGHT: Rgb = [0x47, 0xf0, 0xf3];
const TEAL: Rgb = [0x21, 0x9c, 0xb4];
const TEAL_DEEP: Rgb = [0x0b, 0x6c, 0x8f];

/// The banner is softened a little so the game details stay readable on top of it.
const BANNER_BLUR: f32 = 3.0;
const BANNER_SATURATION: f32 = 1.2;
/// Background when the event has no banner: a dark blue diagonal gradient.
const PLAIN_BACKGROUND: [Rgb; 2] = [[0x0b, 0x1d, 0x33], [0x12, 0x3a, 0x5c]];

/// How far the bars and name plates lean, in degrees.
const SLANT_DEG: f32 = -12.0;
/// Lean of the team names and "VS". The font has no italic, so it is drawn slanted.
const ITALIC: f32 = 0.25;

/// The two halves: how much of the width each covers, and how dark/light they are.
const HALF_WIDTH: f32 = 0.62;
const HALF_SHADE: [f32; 3] = [0.72, 0.5, 0.0];
const HALF_TINT: [f32; 3] = [0.66, 0.45, 0.0];
const HALF_DARK: Rgb = [5, 8, 12];
const HALF_LIGHT: Rgb = [245, 248, 252];

const SLASH_WIDTH: f32 = 18.0;
const SLASH_ANGLE: f32 = 13.0;

/// Height of the middle of "VS" and of the name plate of a team without a logo: the middle of
/// the picture.
const MIDDLE_Y: f32 = HEIGHT as f32 / 2.0;
/// Space between a logo and its name plate.
const PLATE_GAP: f32 = 12.0;
/// A name plate's padding above and below its text.
const PLATE_PAD: f32 = 8.0;
/// A name plate's line spacing, as a multiple of the text size.
const PLATE_LINE_SPACING: f32 = 1.1;
/// The height of a name plate's teal edge.
const PLATE_EDGE: f32 = 5.0;
/// A logo at least this many times wider than it is tall counts as wide.
const WIDE_LOGO: f32 = 1.4;

const NAME_SIZE: f32 = 36.0;
const NAME_MAX_WIDTH: f32 = 440.0;
const NAME_MAX_LINES: usize = 2;
/// Narrower for the plate of a team without a logo, which sits beside "VS".
const NAME_ONLY_MAX_WIDTH: f32 = 360.0;
const SMALLEST_TEXT: f32 = 20.0;

/// The band YouTube always shows: on an upcoming video's page in a wide window (about 3:1) it
/// crops the picture's top and bottom to fill the player, so everything that matters is
/// drawn between these heights and only the background reaches the edges.
const SAFE_TOP: f32 = 145.0;
const SAFE_BOTTOM: f32 = 575.0;

/// The event name's largest size; a long name shrinks.
const EVENT_NAME_SIZE: f32 = 28.0;
/// The bottom bar's largest text size; a long line shrinks.
const INFO_SIZE: f32 = 36.0;

const BRAND_LEFT: f32 = 34.0;
const BRAND_TOP: f32 = SAFE_TOP;
const BRAND_HEIGHT: f32 = 70.0;

struct SideStyle {
    center_x: f32,
    /// How dark the shadow behind the logo is.
    shadow: f32,
    plate: [Rgb; 2],
}

const DARK_SIDE: SideStyle = SideStyle {
    center_x: 305.0,
    shadow: 0.7,
    plate: [INK, WHITE],
};

const LIGHT_SIDE: SideStyle = SideStyle {
    center_x: WIDTH as f32 - 305.0,
    shadow: 0.5,
    plate: [PAPER, INK],
};

/// The shape of the logo boxes, the same for every game of an event: square for events whose
/// team logos are square (1:1), wide for events whose logos are wide (2:1), e.g. flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogoShape {
    Square,
    Wide,
}

impl LogoShape {
    /// How many of the logo pictures are wide (as uploaded, e.g. 1000×500 is wide), and how
    /// many there are.
    pub fn count<'a>(logos: impl IntoIterator<Item = &'a DynamicImage>) -> (usize, usize) {
        let (mut wide, mut total) = (0, 0);
        for logo in logos {
            total += 1;
            if logo.width() as f32 >= logo.height() as f32 * WIDE_LOGO {
                wide += 1;
            }
        }
        (wide, total)
    }

    /// The shape of `total` logos of which `wide` are wide: wide only when most are, so square
    /// when there are none.
    pub fn majority(wide: usize, total: usize) -> Self {
        if wide * 2 > total {
            Self::Wide
        } else {
            Self::Square
        }
    }

    /// The [`Self::majority`] of `wide` of `total` logos plus `unread` more of unknown shape,
    /// if it is the same whatever shape those turn out to be.
    pub fn certain(wide: usize, total: usize, unread: usize) -> Option<Self> {
        let all_square = Self::majority(wide, total + unread);
        let all_wide = Self::majority(wide + unread, total + unread);
        (all_square == all_wide).then_some(all_square)
    }

    /// Half the box's width and height.
    fn half(self) -> (f32, f32) {
        match self {
            Self::Square => (94.0, 94.0),
            Self::Wide => (174.0, 94.0),
        }
    }
}

// ---- What gets drawn ----

pub struct Team<'a> {
    pub name: &'a str,
    /// `None` shows the team's name in place of the logo.
    pub logo: Option<&'a DynamicImage>,
}

pub struct Game<'a> {
    pub number: &'a str,
    pub court: &'a str,
    /// e.g. "Wed 5 Aug, 2:00 PM".
    pub when: &'a str,
    pub dark: Team<'a>,
    pub light: Team<'a>,
}

/// A thumbnail that isn't drawn because the font has no letter for a character it would show.
/// The picture is skipped rather than drawn with an empty box where that letter should be.
#[derive(Debug)]
pub struct MissingGlyph {
    pub ch: char,
    /// In the event name, which is on every game's thumbnail.
    pub in_event_name: bool,
}

impl std::fmt::Display for MissingGlyph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the font has no \"{}\"", self.ch)?;
        if self.in_event_name {
            write!(f, " in the event name")?;
        }
        Ok(())
    }
}

impl std::error::Error for MissingGlyph {}

/// Draws thumbnails for one event. Everything that is the same for every game (banner,
/// halves, event name, "VS", logo) is drawn once up front.
pub struct Painter {
    base: Pixmap,
    font: Font,
    shape: LogoShape,
}

impl Painter {
    pub fn new(
        event_name: &str,
        banner: Option<&DynamicImage>,
        shape: LogoShape,
    ) -> Result<Self, BoxError> {
        let font = Font::new()?;
        // Checked as drawn, in capitals: a letter's capital may be a different character.
        font.drawable(&event_name.to_uppercase(), true)?;
        let mut base = background(banner).ok_or("could not create the thumbnail canvas")?;
        halves(&mut base);
        slash(&mut base);
        event_bar(&mut base, &font, event_name);
        vs(&mut base, &font);
        brand(&mut base)?;
        Ok(Self { base, font, shape })
    }

    /// The finished thumbnail as a JPEG.
    pub fn render(&self, game: &Game) -> Result<Vec<u8>, BoxError> {
        for team in [&game.dark, &game.light] {
            self.font.drawable(&name_text(team), false)?;
        }
        let (head, rest) = info_text(game);
        self.font.drawable(&head, false)?;
        self.font.drawable(&rest, false)?;
        let mut canvas = self.base.clone();
        // Both logos sit at the same height, set by the taller of the two name plates.
        let plate = [&game.dark, &game.light]
            .iter()
            .filter(|t| t.logo.is_some())
            .map(|t| plate_height(&self.font, t))
            .fold(0.0, f32::max);
        let cy = logo_centre_y(&self.font, self.shape, plate);
        side(
            &mut canvas,
            &self.font,
            &game.dark,
            &DARK_SIDE,
            self.shape,
            cy,
        );
        side(
            &mut canvas,
            &self.font,
            &game.light,
            &LIGHT_SIDE,
            self.shape,
            cy,
        );
        info_bar(&mut canvas, &self.font, game);
        encode_jpeg(&canvas)
    }
}

// ---- Layers ----

fn background(banner: Option<&DynamicImage>) -> Option<Pixmap> {
    let mut canvas = Pixmap::new(WIDTH, HEIGHT)?;
    let Some(banner) = banner else {
        let shader = css_gradient(
            (0.0, 0.0, WIDTH as f32, HEIGHT as f32),
            135.0,
            vec![
                GradientStop::new(0.0, rgba(PLAIN_BACKGROUND[0], 1.0)),
                GradientStop::new(1.0, rgba(PLAIN_BACKGROUND[1], 1.0)),
            ],
            Transform::identity(),
        )?;
        let rect = Rect::from_xywh(0.0, 0.0, WIDTH as f32, HEIGHT as f32)?;
        canvas.fill_rect(rect, &paint(shader), Transform::identity(), None);
        return Some(canvas);
    };
    // Cover a slightly larger area and cut out the middle, so the blur has no soft edges.
    let pad = 30;
    let cover = banner
        .resize_to_fill(WIDTH + 2 * pad, HEIGHT + 2 * pad, FilterType::Triangle)
        .to_rgba8();
    let blurred = imageops::blur(&cover, BANNER_BLUR);
    let cropped = imageops::crop_imm(&blurred, pad, pad, WIDTH, HEIGHT).to_image();
    for (px, src) in canvas.pixels_mut().iter_mut().zip(cropped.pixels()) {
        let [r, g, b, a] = src.0;
        // A see-through banner sits on the plain background colour.
        let mix = |c: u8, under: u8| {
            let a = f32::from(a) / 255.0;
            (f32::from(c) * a + f32::from(under) * (1.0 - a)).round() as u8
        };
        let under = PLAIN_BACKGROUND[0];
        let [r, g, b] = saturate(
            [mix(r, under[0]), mix(g, under[1]), mix(b, under[2])],
            BANNER_SATURATION,
        );
        *px = ColorU8::from_rgba(r, g, b, 255).premultiply();
    }
    Some(canvas)
}

/// The same colour boost as CSS `saturate()`.
fn saturate([r, g, b]: Rgb, s: f32) -> Rgb {
    let (r, g, b) = (f32::from(r), f32::from(g), f32::from(b));
    [
        [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
    ]
    .map(|m| (m[0] * r + m[1] * g + m[2] * b).round().clamp(0.0, 255.0) as u8)
}

/// The see-through black (left) and white (right) halves that meet along the slash.
fn halves(canvas: &mut Pixmap) {
    let (w, h) = (WIDTH as f32 * HALF_WIDTH, HEIGHT as f32);
    let stops = |color: Rgb, alpha: [f32; 3]| {
        vec![
            GradientStop::new(0.0, rgba(color, alpha[0])),
            GradientStop::new(0.7, rgba(color, alpha[1])),
            GradientStop::new(1.0, rgba(color, alpha[2])),
        ]
    };
    let dark = polygon(&[(0.0, 0.0), (w, 0.0), (w * 0.78, h), (0.0, h)]);
    let shader = css_gradient(
        (0.0, 0.0, w, h),
        100.0,
        stops(HALF_DARK, HALF_SHADE),
        Transform::identity(),
    );
    if let (Some(path), Some(shader)) = (dark, shader) {
        fill(canvas, &path, paint(shader), Transform::identity());
    }
    let x = WIDTH as f32 - w;
    let light = polygon(&[(x + w * 0.22, 0.0), (x + w, 0.0), (x + w, h), (x, h)]);
    let shader = css_gradient(
        (x, 0.0, w, h),
        260.0,
        stops(HALF_LIGHT, HALF_TINT),
        Transform::identity(),
    );
    if let (Some(path), Some(shader)) = (light, shader) {
        fill(canvas, &path, paint(shader), Transform::identity());
    }
}

fn slash(canvas: &mut Pixmap) {
    let (top, bottom) = (-40.0, HEIGHT as f32 + 40.0);
    let Some(rect) = Rect::from_xywh(CENTER_X - SLASH_WIDTH / 2.0, top, SLASH_WIDTH, bottom - top)
    else {
        return;
    };
    let path = PathBuilder::from_rect(rect);
    let turn = Transform::from_rotate_at(SLASH_ANGLE, CENTER_X, HEIGHT as f32 / 2.0);
    shadow(canvas, &path, turn, Glow::new(0.0, 40.0, TEAL_LIGHT, 0.8));
    let shader = LinearGradient::new(
        Point::from_xy(CENTER_X, top),
        Point::from_xy(CENTER_X, bottom),
        vec![
            GradientStop::new(0.0, rgba(TEAL_LIGHT, 1.0)),
            GradientStop::new(1.0, rgba(TEAL, 1.0)),
        ],
        SpreadMode::Pad,
        turn,
    );
    if let Some(shader) = shader {
        fill(canvas, &path, paint(shader), turn);
    }
}

/// The event name on a teal bar at the top.
fn event_bar(canvas: &mut Pixmap, font: &Font, event_name: &str) {
    let spacing = 2.0;
    let (size, lines) = font.fit(
        &event_name.to_uppercase(),
        EVENT_NAME_SIZE,
        spacing,
        1000.0,
        1,
    );
    let text = lines.join(" ");
    let line = font.normal_line(size);
    let (w, h) = (font.width(&text, size, spacing) + 68.0, line + 16.0);
    let (x, top) = (CENTER_X - w / 2.0, SAFE_TOP);
    let lean = slanted(top + h / 2.0);
    if let Some(rect) = Rect::from_xywh(x, top, w, h) {
        let path = PathBuilder::from_rect(rect);
        shadow(canvas, &path, lean, Glow::new(6.0, 20.0, BLACK, 0.5));
        fill(canvas, &path, solid(TEAL_DEEP, 1.0), lean);
    }
    let baseline = font.baseline(top + 8.0, size, line);
    if let Some(path) = font.path(&text, x + 34.0, baseline, size, spacing, 0.0) {
        fill(canvas, &path, solid(WHITE, 1.0), Transform::identity());
    }
}

/// "VS", in the middle of the picture.
fn vs(canvas: &mut Pixmap, font: &Font) {
    let size = 120.0;
    let x = CENTER_X - font.width("VS", size, 0.0) / 2.0;
    let baseline = MIDDLE_Y + font.cap_height(size) / 2.0;
    let Some(path) = font.path("VS", x, baseline, size, 0.0, ITALIC) else {
        return;
    };
    let lean = slanted(MIDDLE_Y);
    shadow(canvas, &path, lean, Glow::new(0.0, 30.0, TEAL_LIGHT, 0.5));
    shadow(canvas, &path, lean, Glow::new(6.0, 24.0, BLACK, 0.8));
    fill(canvas, &path, solid(TEAL_LIGHT, 1.0), lean);
    let stroke = Stroke {
        width: 4.0,
        ..Stroke::default()
    };
    canvas.stroke_path(&path, &solid(BLACK, 1.0), &stroke, lean, None);
}

/// The small Atlantis Sports logo in the top-left corner.
fn brand(canvas: &mut Pixmap) -> Result<(), BoxError> {
    let logo = image::load_from_memory(BRAND_LOGO)?;
    let width = (logo.width() as f32 * BRAND_HEIGHT / logo.height() as f32).round() as u32;
    let logo = logo
        .resize_exact(width, BRAND_HEIGHT as u32, FilterType::Lanczos3)
        .to_rgba8();
    let logo = to_pixmap(&logo).ok_or("could not prepare the Atlantis logo")?;
    let (x, y) = (BRAND_LEFT as i32, BRAND_TOP as i32);
    // Its shadow follows the logo's own outline.
    if let Some(mut layer) = Pixmap::new(WIDTH, HEIGHT) {
        layer.draw_pixmap(
            x,
            y + 2,
            logo.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        let mask = Mask::from_pixmap(layer.as_ref(), MaskType::Alpha);
        shadow_from_mask(canvas, &mask, Glow::new(0.0, 6.0, BLACK, 0.8));
    }
    let look = PixmapPaint {
        opacity: 0.95,
        ..PixmapPaint::default()
    };
    canvas.draw_pixmap(x, y, logo.as_ref(), &look, Transform::identity(), None);
    Ok(())
}

/// One team: its logo in a box with the name on a plate below, or just its name when it has
/// no logo.
fn side(
    canvas: &mut Pixmap,
    font: &Font,
    team: &Team,
    style: &SideStyle,
    shape: LogoShape,
    cy: f32,
) {
    let Some(logo) = team.logo else {
        name_only(canvas, font, team, style);
        return;
    };
    let cx = style.center_x;
    let (hw, hh) = shape.half();
    team_logo(canvas, logo, (cx, cy), (hw, hh), style.shadow);

    let top = cy + hh + PLATE_GAP;
    let fit = (NAME_SIZE, NAME_MAX_WIDTH, NAME_MAX_LINES);
    name_plate(canvas, font, team, style, fit, |_| top);
}

/// A team without a logo (or a placeholder like "Winner G52"): the same name plate as under a
/// logo, level with the middle of the other team's logo and a little narrower so it stays
/// clear of "VS".
fn name_only(canvas: &mut Pixmap, font: &Font, team: &Team, style: &SideStyle) {
    let fit = (NAME_SIZE, NAME_ONLY_MAX_WIDTH, NAME_MAX_LINES);
    name_plate(canvas, font, team, style, fit, |h| MIDDLE_Y - h / 2.0);
}

/// Height of the name plate under a logo.
fn plate_height(font: &Font, team: &Team) -> f32 {
    let (size, lines) = font.fit(
        &name_text(team),
        NAME_SIZE,
        0.0,
        NAME_MAX_WIDTH,
        NAME_MAX_LINES,
    );
    plate_height_for(size, lines.len())
}

/// Height of the middle of the logo boxes: a logo and its name plate (`plate` tall) are centred
/// together between the event bar and the bottom bar, both at their tallest.
fn logo_centre_y(font: &Font, shape: LogoShape, plate: f32) -> f32 {
    let (_, hh) = shape.half();
    let block = 2.0 * hh + PLATE_GAP + plate;
    (event_bar_bottom(font) + info_bar_top(font)) / 2.0 - block / 2.0 + hh
}

/// The lowest the event bar reaches: its bottom at the largest text size.
fn event_bar_bottom(font: &Font) -> f32 {
    SAFE_TOP + font.normal_line(EVENT_NAME_SIZE) + 16.0
}

/// The highest the bottom bar reaches: its top at the largest text size.
fn info_bar_top(font: &Font) -> f32 {
    SAFE_BOTTOM - (font.normal_line(INFO_SIZE) + 24.0)
}

/// Height of a name plate holding `lines` lines of text at `size`.
fn plate_height_for(size: f32, lines: usize) -> f32 {
    2.0 * PLATE_PAD + size * PLATE_LINE_SPACING * lines as f32 + PLATE_EDGE
}

/// The team's name on a slanted plate with a teal edge: as large as `largest` allows within
/// `max_width` and `max_lines`, with its top where `top` puts it for the plate's height.
fn name_plate(
    canvas: &mut Pixmap,
    font: &Font,
    team: &Team,
    style: &SideStyle,
    (largest, max_width, max_lines): (f32, f32, usize),
    top: impl FnOnce(f32) -> f32,
) {
    let cx = style.center_x;
    let pad = PLATE_PAD;
    let (size, lines) = font.fit(&name_text(team), largest, 0.0, max_width, max_lines);
    let line = size * PLATE_LINE_SPACING;
    let text_width = lines
        .iter()
        .map(|l| font.width(l, size, 0.0))
        .fold(0.0, f32::max);
    let (w, h) = (text_width + 52.0, plate_height_for(size, lines.len()));
    let top = top(h);
    let lean = slanted(top + h / 2.0);
    if let Some(plate) = Rect::from_xywh(cx - w / 2.0, top, w, h) {
        let plate = PathBuilder::from_rect(plate);
        shadow(canvas, &plate, lean, Glow::new(8.0, 22.0, BLACK, 0.55));
        fill(canvas, &plate, solid(style.plate[0], 1.0), lean);
    }
    if let Some(edge) = Rect::from_xywh(cx - w / 2.0, top + h - PLATE_EDGE, w, PLATE_EDGE) {
        fill(
            canvas,
            &PathBuilder::from_rect(edge),
            solid(TEAL, 1.0),
            lean,
        );
    }
    for (i, text) in lines.iter().enumerate() {
        let x = cx - font.width(text, size, 0.0) / 2.0;
        let baseline = font.baseline(top + pad + line * i as f32, size, line);
        if let Some(path) = font.path(text, x, baseline, size, 0.0, ITALIC) {
            fill(
                canvas,
                &path,
                solid(style.plate[1], 1.0),
                Transform::identity(),
            );
        }
    }
}

/// The team name as its plate shows it.
fn name_text(team: &Team) -> String {
    team.name.to_uppercase()
}

/// A team logo as uploaded (no frame or backdrop), as large as fits its space, centred on
/// (`cx`, `cy`) with a soft shadow that follows its outline.
fn team_logo(
    canvas: &mut Pixmap,
    logo: &DynamicImage,
    (cx, cy): (f32, f32),
    (hw, hh): (f32, f32),
    shade: f32,
) {
    let scale = (2.0 * hw / logo.width().max(1) as f32).min(2.0 * hh / logo.height().max(1) as f32);
    let w = ((logo.width() as f32 * scale).round() as u32).max(1);
    let h = ((logo.height() as f32 * scale).round() as u32).max(1);
    let Some(picture) = to_pixmap(&logo.resize_exact(w, h, FilterType::Lanczos3).to_rgba8()) else {
        return;
    };
    let (x, y) = (
        (cx - w as f32 / 2.0).round() as i32,
        (cy - h as f32 / 2.0).round() as i32,
    );
    // Its shadow follows the logo's own outline, so a see-through crest gets no dark box.
    if let Some(mut layer) = Pixmap::new(WIDTH, HEIGHT) {
        layer.draw_pixmap(
            x,
            y + 14,
            picture.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        let mask = Mask::from_pixmap(layer.as_ref(), MaskType::Alpha);
        shadow_from_mask(canvas, &mask, Glow::new(0.0, 40.0, BLACK, shade));
    }
    canvas.draw_pixmap(
        x,
        y,
        picture.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

/// "GAME 12   Court 1 · Wed 5 Aug, 2:00 PM" on a dark bar at the bottom.
fn info_bar(canvas: &mut Pixmap, font: &Font, game: &Game) {
    let (head, rest) = info_text(game);
    let gap = 16.0;
    let mut size = INFO_SIZE;
    let total = |size: f32| font.width(&head, size, 0.0) + gap + font.width(&rest, size, 0.0);
    while total(size) > 1100.0 && size > SMALLEST_TEXT {
        size -= 2.0;
    }
    let line = font.normal_line(size);
    let (w, h) = (total(size) + 80.0 + 20.0, line + 24.0);
    let (x, top) = (CENTER_X - w / 2.0, SAFE_BOTTOM - h);
    let lean = slanted(top + h / 2.0);
    if let Some(bar) = Rect::from_xywh(x, top, w, h) {
        let bar = PathBuilder::from_rect(bar);
        fill(canvas, &bar, solid(BLACK, 0.85), lean);
    }
    for edge_x in [x, x + w - 10.0] {
        if let Some(edge) = Rect::from_xywh(edge_x, top, 10.0, h) {
            fill(
                canvas,
                &PathBuilder::from_rect(edge),
                solid(TEAL, 1.0),
                lean,
            );
        }
    }
    let baseline = font.baseline(top + 12.0, size, line);
    let x = x + 50.0;
    if let Some(path) = font.path(&head, x, baseline, size, 0.0, 0.0) {
        fill(canvas, &path, solid(TEAL_LIGHT, 1.0), Transform::identity());
    }
    let x = x + font.width(&head, size, 0.0) + gap;
    if let Some(path) = font.path(&rest, x, baseline, size, 0.0, 0.0) {
        fill(canvas, &path, solid(WHITE, 1.0), Transform::identity());
    }
}

/// The two parts of the bottom bar's text.
fn info_text(game: &Game) -> (String, String) {
    (
        format!("GAME {}", game.number),
        format!("Court {} · {}", game.court, game.when),
    )
}

fn encode_jpeg(canvas: &Pixmap) -> Result<Vec<u8>, BoxError> {
    let rgb: Vec<u8> = canvas
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue()]
        })
        .collect();
    for quality in [90, 80, 70, 60] {
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, quality).encode(
            &rgb,
            WIDTH,
            HEIGHT,
            ExtendedColorType::Rgb8,
        )?;
        if jpeg.len() <= MAX_BYTES {
            return Ok(jpeg);
        }
    }
    Err("the thumbnail came out larger than YouTube's 2 MB limit".into())
}

// ---- Text ----

struct Font {
    face: Face<'static>,
}

impl Font {
    fn new() -> Result<Self, BoxError> {
        let mut face = Face::parse(FONT, 0)?;
        // Bahnschrift is a variable font; pick its bold weight.
        face.set_variation(Tag::from_bytes(b"wght"), 700.0);
        Ok(Self { face })
    }

    fn scale(&self, size: f32) -> f32 {
        size / f32::from(self.face.units_per_em())
    }

    /// Whether the font can draw every character of `text`; if not, the first one it can't.
    /// Spaces and other blanks don't count: there is nothing to see.
    fn covers(&self, text: &str) -> Result<(), char> {
        match text
            .chars()
            .find(|ch| !ch.is_whitespace() && self.face.glyph_index(*ch).is_none())
        {
            Some(ch) => Err(ch),
            None => Ok(()),
        }
    }

    /// Refuses `text` if the font can't draw all of it.
    fn drawable(&self, text: &str, in_event_name: bool) -> Result<(), MissingGlyph> {
        self.covers(text)
            .map_err(|ch| MissingGlyph { ch, in_event_name })
    }

    /// Only a blank can be missing here, since `covers` is checked before anything is drawn
    /// (e.g. a tab in a court name): it takes the place of a space.
    fn glyph(&self, ch: char) -> GlyphId {
        self.face
            .glyph_index(ch)
            .or_else(|| self.face.glyph_index(' '))
            .unwrap_or(GlyphId(0))
    }

    /// Width of `text`, including the extra `spacing` after every letter (as CSS does).
    fn width(&self, text: &str, size: f32, spacing: f32) -> f32 {
        let scale = self.scale(size);
        text.chars()
            .map(|ch| {
                let advance = self.face.glyph_hor_advance(self.glyph(ch)).unwrap_or(0);
                f32::from(advance) * scale + spacing
            })
            .sum()
    }

    /// Height of a capital letter.
    fn cap_height(&self, size: f32) -> f32 {
        let units = self
            .face
            .capital_height()
            .map_or(f32::from(self.face.ascender()) * 0.7, f32::from);
        units * self.scale(size)
    }

    /// Height of a line at the font's own line spacing.
    fn normal_line(&self, size: f32) -> f32 {
        let f = &self.face;
        (f32::from(f.ascender()) - f32::from(f.descender()) + f32::from(f.line_gap()))
            * self.scale(size)
    }

    /// Where the baseline sits for a line `line_height` tall whose top is at `top`.
    fn baseline(&self, top: f32, size: f32, line_height: f32) -> f32 {
        let scale = self.scale(size);
        let ascent = f32::from(self.face.ascender()) * scale;
        let descent = -f32::from(self.face.descender()) * scale;
        top + (line_height - ascent - descent) / 2.0 + ascent
    }

    /// The outline of `text` starting at `x`, leaning right by `italic`.
    fn path(
        &self,
        text: &str,
        x: f32,
        baseline: f32,
        size: f32,
        spacing: f32,
        italic: f32,
    ) -> Option<Path> {
        let mut builder = PathBuilder::new();
        let scale = self.scale(size);
        let mut pen = x;
        for ch in text.chars() {
            let glyph = self.glyph(ch);
            let mut outline = Outline {
                builder: &mut builder,
                x: pen,
                y: baseline,
                scale,
                italic,
            };
            self.face.outline_glyph(glyph, &mut outline);
            pen += f32::from(self.face.glyph_hor_advance(glyph).unwrap_or(0)) * scale + spacing;
        }
        builder.finish()
    }

    /// Splits `text` into lines no wider than `max_width`, shrinking the text if it needs more
    /// than `max_lines` lines or a single word is too wide. Returns the size used and the lines.
    fn fit(
        &self,
        text: &str,
        size: f32,
        spacing: f32,
        max_width: f32,
        max_lines: usize,
    ) -> (f32, Vec<String>) {
        let mut size = size;
        loop {
            let lines = self.wrap(text, size, spacing, max_width);
            let fits = lines.len() <= max_lines
                && lines
                    .iter()
                    .all(|l| self.width(l, size, spacing) <= max_width);
            if fits || size <= SMALLEST_TEXT {
                return (size, lines);
            }
            size -= 2.0;
        }
    }

    fn wrap(&self, text: &str, size: f32, spacing: f32, max_width: f32) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for word in unbreakable_words(text) {
            match lines.last_mut() {
                Some(last) if self.width(&format!("{last} {word}"), size, spacing) <= max_width => {
                    last.push(' ');
                    last.push_str(&word);
                }
                _ => lines.push(word),
            }
        }
        lines
    }
}

/// The words of `text`, with a single letter kept on the same line as the word after it
/// ("A TEAM"), or the word before it when it comes last ("TEAM B").
fn unbreakable_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut letter: Option<String> = None;
    for word in text.split_whitespace() {
        let word = match letter.take() {
            Some(letter) => format!("{letter} {word}"),
            None => word.to_string(),
        };
        if word.chars().count() == 1 {
            letter = Some(word);
        } else {
            words.push(word);
        }
    }
    if let Some(letter) = letter {
        match words.last_mut() {
            Some(last) => {
                last.push(' ');
                last.push_str(&letter);
            }
            None => words.push(letter),
        }
    }
    words
}

/// Turns a glyph's outline (font units, y up) into canvas coordinates (pixels, y down).
struct Outline<'a> {
    builder: &'a mut PathBuilder,
    x: f32,
    y: f32,
    scale: f32,
    italic: f32,
}

impl Outline<'_> {
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.x + (x + self.italic * y) * self.scale,
            self.y - y * self.scale,
        )
    }
}

impl OutlineBuilder for Outline<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.at(x1, y1);
        let (x, y) = self.at(x, y);
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.at(x1, y1);
        let (x2, y2) = self.at(x2, y2);
        let (x, y) = self.at(x, y);
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

// ---- Drawing helpers ----

fn rgba(color: Rgb, alpha: f32) -> Color {
    Color::from_rgba8(color[0], color[1], color[2], (alpha * 255.0).round() as u8)
}

fn paint(shader: Shader<'static>) -> Paint<'static> {
    Paint {
        shader,
        anti_alias: true,
        ..Paint::default()
    }
}

fn solid(color: Rgb, alpha: f32) -> Paint<'static> {
    paint(Shader::SolidColor(rgba(color, alpha)))
}

fn fill(canvas: &mut Pixmap, path: &Path, paint: Paint, transform: Transform) {
    canvas.fill_path(path, &paint, FillRule::Winding, transform, None);
}

/// Leans a shape like CSS `skewX(-12deg)`, pivoting on the horizontal line at `center_y`.
fn slanted(center_y: f32) -> Transform {
    let k = SLANT_DEG.to_radians().tan();
    Transform::from_row(1.0, 0.0, k, 1.0, -k * center_y, 0.0)
}

fn polygon(points: &[(f32, f32)]) -> Option<Path> {
    let mut builder = PathBuilder::new();
    let (first, rest) = points.split_first()?;
    builder.move_to(first.0, first.1);
    for (x, y) in rest {
        builder.line_to(*x, *y);
    }
    builder.close();
    builder.finish()
}

/// A CSS-style `linear-gradient(<angle>deg, …)` across the box `(x, y, width, height)`.
fn css_gradient(
    (x, y, w, h): (f32, f32, f32, f32),
    angle: f32,
    stops: Vec<GradientStop>,
    transform: Transform,
) -> Option<Shader<'static>> {
    let (dx, dy) = (angle.to_radians().sin(), -angle.to_radians().cos());
    let half = ((w * dx).abs() + (h * dy).abs()) / 2.0;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    LinearGradient::new(
        Point::from_xy(cx - dx * half, cy - dy * half),
        Point::from_xy(cx + dx * half, cy + dy * half),
        stops,
        SpreadMode::Pad,
        transform,
    )
}

/// A soft shadow or glow, like CSS `box-shadow: 0 <dy> <blur> <color>`.
#[derive(Clone, Copy)]
struct Glow {
    dy: f32,
    blur: f32,
    color: Rgb,
    alpha: f32,
}

impl Glow {
    fn new(dy: f32, blur: f32, color: Rgb, alpha: f32) -> Self {
        Self {
            dy,
            blur,
            color,
            alpha,
        }
    }
}

fn shadow(canvas: &mut Pixmap, path: &Path, transform: Transform, glow: Glow) {
    let Some(mut mask) = Mask::new(WIDTH, HEIGHT) else {
        return;
    };
    mask.fill_path(
        path,
        FillRule::Winding,
        true,
        transform.post_translate(0.0, glow.dy),
    );
    shadow_from_mask(canvas, &mask, glow);
}

fn shadow_from_mask(canvas: &mut Pixmap, mask: &Mask, glow: Glow) {
    let Some(shape) = GrayImage::from_raw(WIDTH, HEIGHT, mask.data().to_vec()) else {
        return;
    };
    // CSS blur lengths are twice the blur's standard deviation.
    let soft = imageops::fast_blur(&shape, glow.blur / 2.0);
    let Some(mut layer) = Pixmap::new(WIDTH, HEIGHT) else {
        return;
    };
    let [r, g, b] = glow.color;
    for (px, m) in layer.pixels_mut().iter_mut().zip(soft.as_raw()) {
        let a = (f32::from(*m) * glow.alpha).round() as u8;
        *px = ColorU8::from_rgba(r, g, b, a).premultiply();
    }
    canvas.draw_pixmap(
        0,
        0,
        layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn to_pixmap(image: &RgbaImage) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(image.width(), image.height())?;
    for (px, src) in pixmap.pixels_mut().iter_mut().zip(image.pixels()) {
        let [r, g, b, a] = src.0;
        *px = ColorU8::from_rgba(r, g, b, a).premultiply();
    }
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game<'a>(dark: Team<'a>, light: Team<'a>) -> Game<'a> {
        Game {
            number: "12",
            court: "1",
            when: "Wed 5 Aug, 2:00 PM",
            dark,
            light,
        }
    }

    #[test]
    fn the_logo_boxes_take_the_shape_most_of_the_event_logos_have() {
        let logo =
            |w, h| DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, [9, 9, 9, 255].into()));
        let of = |logos: &[&DynamicImage]| {
            let (wide, total) = LogoShape::count(logos.iter().copied());
            LogoShape::majority(wide, total)
        };
        let (square, flag) = (logo(100, 100), logo(200, 100));
        assert_eq!(of(&[]), LogoShape::Square);
        assert_eq!(of(&[&square, &flag]), LogoShape::Square);
        assert_eq!(of(&[&flag, &flag, &square]), LogoShape::Wide);
        // A 3:2 flag counts as wide.
        assert_eq!(of(&[&logo(150, 100)]), LogoShape::Wide);
        // So does one stored a pixel short of 3:2.
        assert_eq!(of(&[&logo(1000, 667)]), LogoShape::Wide);
        // A 4:3 logo counts as square.
        assert_eq!(of(&[&logo(400, 300)]), LogoShape::Square);
    }

    #[test]
    fn the_shape_is_certain_only_when_the_unread_logos_cant_change_it() {
        // 3 wide of 4 read: wide even if the 5th is square.
        assert_eq!(LogoShape::certain(3, 4, 1), Some(LogoShape::Wide));
        // 2 wide of 3 read: a square 4th makes it 2 of 4, square.
        assert_eq!(LogoShape::certain(2, 3, 1), None);
        // 1 wide of 4 read: square even if the 5th is wide.
        assert_eq!(LogoShape::certain(1, 4, 1), Some(LogoShape::Square));
        assert_eq!(LogoShape::certain(1, 2, 1), None);
        assert_eq!(LogoShape::certain(1, 2, 0), Some(LogoShape::Square));
    }

    #[test]
    fn a_single_letter_stays_on_the_line_of_its_neighbour() {
        assert_eq!(
            unbreakable_words("CAIRNS CRAYS A TEAM"),
            ["CAIRNS", "CRAYS", "A TEAM"]
        );
        assert_eq!(unbreakable_words("SYDNEY KINGS B"), ["SYDNEY", "KINGS B"]);
        assert_eq!(unbreakable_words("A B"), ["A B"]);
        assert_eq!(unbreakable_words("Q"), ["Q"]);
    }

    #[test]
    fn long_names_wrap_onto_two_lines_and_shrink_if_needed() {
        let font = Font::new().unwrap();
        let (size, lines) = font.fit("SUNNY COAST SEA DRAGONS", NAME_SIZE, 0.0, NAME_MAX_WIDTH, 2);
        assert_eq!(size, NAME_SIZE);
        assert_eq!(lines.len(), 2);

        let long = "SUPERCALIFRAGILISTICEXPIALIDOCIOUS UNDERWATER HOCKEY CLUB";
        let (size, lines) = font.fit(long, NAME_SIZE, 0.0, NAME_MAX_WIDTH, 2);
        assert!(size < NAME_SIZE);
        assert!(lines.len() <= 2);
        assert!(
            lines
                .iter()
                .all(|l| font.width(l, size, 0.0) <= NAME_MAX_WIDTH)
        );
    }

    #[test]
    fn a_see_through_logo_casts_no_shadow_outside_its_artwork() {
        let mut canvas = Pixmap::new(WIDTH, HEIGHT).unwrap();
        canvas.fill(Color::WHITE);
        // A crest that is transparent except for a 20×20 square in its middle.
        let crest = RgbaImage::from_fn(188, 188, |x, y| {
            let inside = (84..104).contains(&x) && (84..104).contains(&y);
            if inside { [9, 9, 9, 255] } else { [0, 0, 0, 0] }.into()
        });
        let crest = DynamicImage::ImageRgba8(crest);
        team_logo(&mut canvas, &crest, (640.0, 360.0), (94.0, 94.0), 0.7);
        let at = |x, y| canvas.pixel(x, y).unwrap();
        // Inside the logo's box, far from the artwork: untouched.
        let corner = at(550, 450);
        assert!(
            [corner.red(), corner.green(), corner.blue()]
                .iter()
                .all(|&c| c > 245),
            "{corner:?}"
        );
        // Just below the artwork: in its shadow (faint, as the artwork is small for the blur).
        let below = at(640, 380);
        assert!(
            [below.red(), below.green(), below.blue()]
                .iter()
                .all(|&c| c < 245),
            "{below:?}"
        );
    }

    #[test]
    fn the_tallest_logo_and_plate_stay_between_the_top_and_bottom_bars() {
        let font = Font::new().unwrap();
        let plate = plate_height_for(NAME_SIZE, NAME_MAX_LINES);
        for shape in [LogoShape::Square, LogoShape::Wide] {
            let (_, hh) = shape.half();
            let cy = logo_centre_y(&font, shape, plate);
            assert!(
                cy - hh >= event_bar_bottom(&font),
                "{shape:?}: logo under the event bar"
            );
            assert!(
                cy + hh + PLATE_GAP + plate <= info_bar_top(&font),
                "{shape:?}: plate under the info bar"
            );
        }
    }

    fn team(name: &str) -> Team<'_> {
        Team { name, logo: None }
    }

    fn missing(result: Result<impl Sized, BoxError>) -> MissingGlyph {
        match result {
            Ok(_) => panic!("drawn, but the font can't draw it"),
            Err(e) => match e.downcast::<MissingGlyph>() {
                Ok(missing) => *missing,
                Err(e) => panic!("failed for another reason: {e}"),
            },
        }
    }

    #[test]
    fn the_font_covers_latin_names_but_not_chinese() {
        let font = Font::new().unwrap();
        assert_eq!(font.covers("Team Alpha"), Ok(()));
        assert_eq!(font.covers("東京"), Err('東'));
        assert_eq!(font.covers("Team 東京"), Err('東'));
        // Blanks are never drawn, even ones the font has no glyph for (tab, ideographic space).
        assert_eq!(font.covers(" \t\u{3000}"), Ok(()));
        // Bahnschrift has these accented letters, in both cases.
        assert_eq!(font.covers("Équipe Sœur"), Ok(()));
        assert_eq!(font.covers("ÉQUIPE SŒUR"), Ok(()));
        // ... but not every Latin letter: "ǆ" (a Croatian digraph) is missing.
        assert_eq!(font.covers("Đurđevac ǆ"), Err('ǆ'));
    }

    #[test]
    fn a_team_name_the_font_cant_draw_skips_the_picture() {
        let painter = Painter::new("Test Cup", None, LogoShape::Square).unwrap();
        let skipped = missing(painter.render(&game(team("東京 Sharks"), team("Sydney Kings A"))));
        assert_eq!(skipped.ch, '東');
        assert!(!skipped.in_event_name);
        assert_eq!(skipped.to_string(), "the font has no \"東\"");

        let skipped = missing(painter.render(&game(team("Sydney Kings A"), team("Ǆ Club"))));
        assert_eq!(skipped.ch, 'Ǆ');

        // The court is on the picture too.
        let mut on_court = game(team("Sydney Kings A"), team("Brisbane"));
        on_court.court = "東";
        assert_eq!(missing(painter.render(&on_court)).ch, '東');
    }

    #[test]
    fn names_are_checked_as_drawn_in_capitals() {
        // "ﬁ" is one ligature character the font lacks, but the name is drawn in capitals,
        // where it becomes the plain letters "FI".
        let painter = Painter::new("Test Cup", None, LogoShape::Square).unwrap();
        let name = "Équipe ﬁnale";
        assert_eq!(Font::new().unwrap().covers(name), Err('ﬁ'));
        assert!(
            painter
                .render(&game(team(name), team("Sydney Kings A")))
                .is_ok()
        );
    }

    #[test]
    fn an_event_name_the_font_cant_draw_skips_every_picture() {
        let skipped = missing(Painter::new("東京 Cup", None, LogoShape::Square));
        assert_eq!(skipped.ch, '東');
        assert!(skipped.in_event_name);
        assert_eq!(
            skipped.to_string(),
            "the font has no \"東\" in the event name"
        );
        // Drawn in capitals: the ligature "ﬁ" becomes "FI", which the font has.
        assert!(Painter::new("ﬁnals", None, LogoShape::Square).is_ok());
    }

    #[test]
    fn renders_a_youtube_sized_jpeg_without_a_banner_or_logos() {
        let painter = Painter::new("Test Cup", None, LogoShape::Square).unwrap();
        let jpeg = painter
            .render(&game(
                Team {
                    name: "Sydney Kings A",
                    logo: None,
                },
                Team {
                    name: "Winner G52",
                    logo: None,
                },
            ))
            .unwrap();
        assert!(jpeg.len() <= MAX_BYTES);
        let image = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((image.width(), image.height()), (WIDTH, HEIGHT));
    }
}
