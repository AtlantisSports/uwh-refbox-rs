//! Draws each game's YouTube thumbnail in the "action" style (Black vs White): the event's
//! portal banner in the background, the dark-cap team on a black half and the light-cap team on
//! a white half, split by a slanted yellow line, with the game details along the bottom.
//!
//! The look is set by the constants below. Change [`DESIGN_VERSION`] whenever the look changes,
//! so that the next Prepare uploads the new design to videos that already have a thumbnail.

use crate::BoxError;
use image::{
    DynamicImage, ExtendedColorType, GrayImage, RgbaImage,
    codecs::jpeg::JpegEncoder,
    imageops::{self, FilterType},
};
use std::borrow::Cow;
use tiny_skia::{
    Color, ColorU8, FillRule, GradientStop, LinearGradient, Mask, MaskType, Paint, Path,
    PathBuilder, Pixmap, PixmapPaint, Point, Rect, Shader, SpreadMode, Stroke, Transform,
};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

/// Bump this whenever the look changes.
pub const DESIGN_VERSION: u32 = 1;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const CENTER_X: f32 = WIDTH as f32 / 2.0;
/// YouTube's size limit for a thumbnail file.
const MAX_BYTES: usize = 2 * 1024 * 1024;

const FONT: &[u8] = include_bytes!("../../overlay/assets/BAHNSCHRIFT.TTF");
const BRAND_LOGO: &[u8] = include_bytes!("../../overlay/assets/color/1080/Atlantis Logo.png");

// ---- Design ----

type Rgb = [u8; 3];
const BLACK: Rgb = [0, 0, 0];
const WHITE: Rgb = [255, 255, 255];
const INK: Rgb = [0x07, 0x09, 0x0c];
const PAPER: Rgb = [0xf7, 0xf9, 0xfb];
const YELLOW: Rgb = [0xff, 0xd2, 0x3f];
const ORANGE: Rgb = [0xff, 0x8a, 0x00];

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

const BADGE_TOP: f32 = 120.0;
const BADGE_RADIUS: f32 = 150.0;
const BADGE_BORDER: f32 = 8.0;
const BADGE_RING: f32 = 6.0;
/// Share of the badge the team logo fills.
const LOGO_FILL: f32 = 0.82;
const INITIALS_COLORS: [Rgb; 2] = [[0x1f, 0x6f, 0xb2], [0x0d, 0x3b, 0x66]];

const NAME_SIZE: f32 = 42.0;
const NAME_MAX_WIDTH: f32 = 440.0;
const NAME_MAX_LINES: usize = 2;
const SMALLEST_TEXT: f32 = 20.0;

const BRAND_LEFT: f32 = 34.0;
const BRAND_TOP: f32 = 24.0;
const BRAND_HEIGHT: f32 = 92.0;

struct SideStyle {
    label: &'static str,
    center_x: f32,
    border: Rgb,
    ring: Rgb,
    shadow: f32,
    tag: [Rgb; 2],
    plate: [Rgb; 2],
}

const DARK_SIDE: SideStyle = SideStyle {
    label: "DARK",
    center_x: 305.0,
    border: [0x11, 0x11, 0x11],
    ring: WHITE,
    shadow: 0.7,
    tag: [WHITE, BLACK],
    plate: [INK, WHITE],
};

const LIGHT_SIDE: SideStyle = SideStyle {
    label: "LIGHT",
    center_x: WIDTH as f32 - 305.0,
    border: WHITE,
    ring: [0x11, 0x11, 0x11],
    shadow: 0.5,
    tag: [BLACK, WHITE],
    plate: [PAPER, INK],
};

// ---- What gets drawn ----

pub struct Team<'a> {
    pub name: &'a str,
    /// False for placeholders like "Winner G52": they get a "?" instead of initials.
    pub known: bool,
    pub logo: Option<&'a DynamicImage>,
}

pub struct Game<'a> {
    pub number: &'a str,
    pub court: &'a str,
    /// e.g. "Wed 5 Aug, 14:00".
    pub when: &'a str,
    pub dark: Team<'a>,
    pub light: Team<'a>,
}

/// Draws thumbnails for one event. Everything that is the same for every game (banner,
/// halves, event name, "VS", logo) is drawn once up front.
pub struct Painter {
    base: Pixmap,
    font: Font,
}

impl Painter {
    pub fn new(event_name: &str, banner: Option<&DynamicImage>) -> Result<Self, BoxError> {
        let font = Font::new()?;
        let mut base = background(banner).ok_or("could not create the thumbnail canvas")?;
        halves(&mut base);
        slash(&mut base);
        event_bar(&mut base, &font, event_name);
        vs(&mut base, &font);
        brand(&mut base)?;
        Ok(Self { base, font })
    }

    /// The finished thumbnail as a JPEG.
    pub fn render(&self, game: &Game) -> Result<Vec<u8>, BoxError> {
        let mut canvas = self.base.clone();
        side(&mut canvas, &self.font, &game.dark, &DARK_SIDE);
        side(&mut canvas, &self.font, &game.light, &LIGHT_SIDE);
        info_bar(&mut canvas, &self.font, game);
        encode_jpeg(&canvas)
    }
}

/// Up to two letters standing for a team without a logo: "Townsville Tigersharks B" → "TT".
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split_whitespace()
        .filter(|w| w.chars().next().is_some_and(char::is_alphanumeric))
        .collect();
    let letters: String = match words.as_slice() {
        [] => String::new(),
        [one] => one
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(2)
            .collect(),
        [first, second, ..] => [first, second]
            .iter()
            .filter_map(|w| w.chars().next())
            .collect(),
    };
    letters.to_uppercase()
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
    shadow(
        canvas,
        &path,
        turn,
        Glow::new(0.0, 40.0, [255, 190, 40], 0.9),
    );
    let shader = LinearGradient::new(
        Point::from_xy(CENTER_X, top),
        Point::from_xy(CENTER_X, bottom),
        vec![
            GradientStop::new(0.0, rgba(YELLOW, 1.0)),
            GradientStop::new(1.0, rgba(ORANGE, 1.0)),
        ],
        SpreadMode::Pad,
        turn,
    );
    if let Some(shader) = shader {
        fill(canvas, &path, paint(shader), turn);
    }
}

/// The event name on a yellow bar at the top.
fn event_bar(canvas: &mut Pixmap, font: &Font, event_name: &str) {
    let spacing = 2.0;
    let (size, lines) = font.fit(&event_name.to_uppercase(), 28.0, spacing, 1000.0, 1);
    let text = lines.join(" ");
    let line = font.normal_line(size);
    let (w, h) = (font.width(&text, size, spacing) + 68.0, line + 16.0);
    let (x, top) = (CENTER_X - w / 2.0, 26.0);
    let lean = slanted(top + h / 2.0);
    if let Some(rect) = Rect::from_xywh(x, top, w, h) {
        let path = PathBuilder::from_rect(rect);
        shadow(canvas, &path, lean, Glow::new(6.0, 20.0, BLACK, 0.5));
        fill(canvas, &path, solid(YELLOW, 1.0), lean);
    }
    let baseline = font.baseline(top + 8.0, size, line);
    if let Some(path) = font.path(&text, x + 34.0, baseline, size, spacing, 0.0) {
        fill(canvas, &path, solid(BLACK, 1.0), Transform::identity());
    }
}

fn vs(canvas: &mut Pixmap, font: &Font) {
    let (size, top) = (140.0, 210.0);
    let line = font.normal_line(size);
    let x = CENTER_X - font.width("VS", size, 0.0) / 2.0;
    let baseline = font.baseline(top, size, line);
    let Some(path) = font.path("VS", x, baseline, size, 0.0, ITALIC) else {
        return;
    };
    let lean = slanted(top + line / 2.0);
    shadow(
        canvas,
        &path,
        lean,
        Glow::new(0.0, 30.0, [255, 180, 0], 0.6),
    );
    shadow(canvas, &path, lean, Glow::new(6.0, 24.0, BLACK, 0.8));
    fill(canvas, &path, solid(YELLOW, 1.0), lean);
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

/// One team: badge (logo or initials), DARK/LIGHT tag and name plate.
fn side(canvas: &mut Pixmap, font: &Font, team: &Team, style: &SideStyle) {
    let cx = style.center_x;
    let outer = BADGE_RADIUS + BADGE_BORDER;
    let cy = BADGE_TOP + outer;
    let id = Transform::identity();

    if let Some(circle) = PathBuilder::from_circle(cx, cy, outer) {
        shadow(
            canvas,
            &circle,
            id,
            Glow::new(14.0, 40.0, BLACK, style.shadow),
        );
    }
    if let Some(ring) = PathBuilder::from_circle(cx, cy, outer + BADGE_RING) {
        fill(canvas, &ring, solid(style.ring, 1.0), id);
    }
    if let Some(border) = PathBuilder::from_circle(cx, cy, outer) {
        fill(canvas, &border, solid(style.border, 1.0), id);
    }
    if let Some(inner) = PathBuilder::from_circle(cx, cy, BADGE_RADIUS) {
        match team.logo {
            Some(logo) => {
                let (logo, backdrop) = badge_logo(logo);
                fill(canvas, &inner, solid(backdrop, 1.0), id);
                team_logo(canvas, &logo, cx, cy, &inner);
            }
            None => {
                let r = BADGE_RADIUS;
                let gradient = css_gradient(
                    (cx - r, cy - r, 2.0 * r, 2.0 * r),
                    135.0,
                    vec![
                        GradientStop::new(0.0, rgba(INITIALS_COLORS[0], 1.0)),
                        GradientStop::new(1.0, rgba(INITIALS_COLORS[1], 1.0)),
                    ],
                    id,
                );
                if let Some(shader) = gradient {
                    fill(canvas, &inner, paint(shader), id);
                }
                let letters = if team.known {
                    initials(team.name)
                } else {
                    "?".to_string()
                };
                let (size, spacing) = (110.0, 2.0);
                let line = font.normal_line(size);
                let x = cx - font.width(&letters, size, spacing) / 2.0;
                let baseline = font.baseline(cy - line / 2.0, size, line);
                if let Some(path) = font.path(&letters, x, baseline, size, spacing, 0.0) {
                    fill(canvas, &path, solid(WHITE, 1.0), id);
                }
            }
        }
    }

    // DARK / LIGHT tag.
    let (size, spacing) = (24.0, 3.0);
    let line = font.normal_line(size);
    let tag_top = cy + outer + 18.0;
    let (w, h) = (font.width(style.label, size, spacing) + 32.0, line + 8.0);
    if let Some(tag) = rounded_rect(cx - w / 2.0, tag_top, w, h, 6.0) {
        fill(canvas, &tag, solid(style.tag[0], 1.0), id);
    }
    let baseline = font.baseline(tag_top + 4.0, size, line);
    if let Some(path) = font.path(
        style.label,
        cx - w / 2.0 + 16.0,
        baseline,
        size,
        spacing,
        0.0,
    ) {
        fill(canvas, &path, solid(style.tag[1], 1.0), id);
    }

    // Name plate.
    let (size, lines) = font.fit(
        &team.name.to_uppercase(),
        NAME_SIZE,
        0.0,
        NAME_MAX_WIDTH,
        NAME_MAX_LINES,
    );
    let line = size * 1.1;
    let text_width = lines
        .iter()
        .map(|l| font.width(l, size, 0.0))
        .fold(0.0, f32::max);
    let top = tag_top + h + 10.0;
    let (w, h) = (text_width + 52.0, 16.0 + line * lines.len() as f32 + 5.0);
    let lean = slanted(top + h / 2.0);
    if let Some(plate) = Rect::from_xywh(cx - w / 2.0, top, w, h) {
        let plate = PathBuilder::from_rect(plate);
        shadow(canvas, &plate, lean, Glow::new(8.0, 22.0, BLACK, 0.55));
        fill(canvas, &plate, solid(style.plate[0], 1.0), lean);
    }
    if let Some(edge) = Rect::from_xywh(cx - w / 2.0, top + h - 5.0, w, 5.0) {
        fill(
            canvas,
            &PathBuilder::from_rect(edge),
            solid(YELLOW, 1.0),
            lean,
        );
    }
    for (i, text) in lines.iter().enumerate() {
        let x = cx - font.width(text, size, 0.0) / 2.0;
        let baseline = font.baseline(top + 8.0 + line * i as f32, size, line);
        if let Some(path) = font.path(text, x, baseline, size, 0.0, ITALIC) {
            fill(canvas, &path, solid(style.plate[1], 1.0), id);
        }
    }
}

/// The logo to put in the badge and the colour to fill the badge with, so a logo on a solid
/// background blends in instead of leaving white gaps:
/// - a plain frame or margin around the artwork is cut away if what's left has a solid edge
///   (e.g. a yellow rectangle saved on white fills the badge yellow);
/// - otherwise a logo whose own edge is one solid colour fills the badge with it (e.g. black);
/// - anything else stays on a white badge.
fn badge_logo(logo: &DynamicImage) -> (Cow<'_, DynamicImage>, Rgb) {
    let trimmed = trim_margin(logo);
    if let Some(color) = edge_color(&trimmed) {
        return (Cow::Owned(trimmed), color);
    }
    (Cow::Borrowed(logo), edge_color(logo).unwrap_or(WHITE))
}

/// The logo without the plain frame or empty margin around it (the colour of its top-left
/// corner, or see-through).
fn trim_margin(logo: &DynamicImage) -> DynamicImage {
    let rgba = logo.to_rgba8();
    let Some(corner) = rgba.pixels().next().map(|p| p.0) else {
        return logo.clone();
    };
    let is_margin = |p: [u8; 4]| {
        if corner[3] < 250 {
            p[3] < 16
        } else {
            p[3] >= 250 && (0..3).all(|c| p[c].abs_diff(corner[c]) <= 24)
        }
    };
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in rgba.enumerate_pixels() {
        if !is_margin(p.0) {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    if left > right || top > bottom {
        return logo.clone();
    }
    DynamicImage::ImageRgba8(
        imageops::crop_imm(&rgba, left, top, right - left + 1, bottom - top + 1).to_image(),
    )
}

/// The colour all around a logo's edge, if it is one solid colour (e.g. a square logo on a
/// black background). The badge is filled with it so the logo blends in instead of leaving
/// white gaps. `None` for see-through or mixed edges, which keep the white badge.
fn edge_color(logo: &DynamicImage) -> Option<Rgb> {
    let size = 64;
    let small = logo
        .resize_exact(size, size, FilterType::Triangle)
        .to_rgba8();
    let edge: Vec<[u8; 4]> = (0..size)
        .flat_map(|i| [(i, 0), (i, size - 1), (0, i), (size - 1, i)])
        .map(|(x, y)| small.get_pixel(x, y).0)
        .collect();
    if edge.iter().any(|p| p[3] < 250) {
        return None;
    }
    let average = |c: usize| {
        let total: u32 = edge.iter().map(|p| u32::from(p[c])).sum();
        (total / edge.len() as u32) as u8
    };
    let mean = [average(0), average(1), average(2)];
    let matching = edge
        .iter()
        .filter(|p| (0..3).all(|c| p[c].abs_diff(mean[c]) <= 24))
        .count();
    // Allow a few stray pixels, e.g. where the artwork touches the edge.
    (matching * 10 >= edge.len() * 9).then_some(mean)
}

/// A team logo, fitted inside the badge and cut to its circle.
fn team_logo(canvas: &mut Pixmap, logo: &DynamicImage, cx: f32, cy: f32, circle: &Path) {
    let space = 2.0 * BADGE_RADIUS * LOGO_FILL;
    let scale = (space / logo.width().max(1) as f32).min(space / logo.height().max(1) as f32);
    let w = ((logo.width() as f32 * scale).round() as u32).max(1);
    let h = ((logo.height() as f32 * scale).round() as u32).max(1);
    let Some(picture) = to_pixmap(&logo.resize_exact(w, h, FilterType::Lanczos3).to_rgba8()) else {
        return;
    };
    let Some(mut clip) = Mask::new(WIDTH, HEIGHT) else {
        return;
    };
    clip.fill_path(circle, FillRule::Winding, true, Transform::identity());
    canvas.draw_pixmap(
        (cx - w as f32 / 2.0).round() as i32,
        (cy - h as f32 / 2.0).round() as i32,
        picture.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        Some(&clip),
    );
}

/// "GAME 12   Court 1 · Wed 5 Aug, 14:00" on a dark bar at the bottom.
fn info_bar(canvas: &mut Pixmap, font: &Font, game: &Game) {
    let head = format!("GAME {}", game.number);
    let rest = format!("Court {} · {}", game.court, game.when);
    let gap = 16.0;
    let mut size: f32 = 36.0;
    let total = |size: f32| font.width(&head, size, 0.0) + gap + font.width(&rest, size, 0.0);
    while total(size) > 1100.0 && size > SMALLEST_TEXT {
        size -= 2.0;
    }
    let line = font.normal_line(size);
    let (w, h) = (total(size) + 80.0 + 20.0, line + 24.0);
    let (x, top) = (CENTER_X - w / 2.0, HEIGHT as f32 - 30.0 - h);
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
                solid(YELLOW, 1.0),
                lean,
            );
        }
    }
    let baseline = font.baseline(top + 12.0, size, line);
    let x = x + 50.0;
    if let Some(path) = font.path(&head, x, baseline, size, 0.0, 0.0) {
        fill(canvas, &path, solid(YELLOW, 1.0), Transform::identity());
    }
    let x = x + font.width(&head, size, 0.0) + gap;
    if let Some(path) = font.path(&rest, x, baseline, size, 0.0, 0.0) {
        fill(canvas, &path, solid(WHITE, 1.0), Transform::identity());
    }
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

    fn glyph(&self, ch: char) -> GlyphId {
        self.face.glyph_index(ch).unwrap_or(GlyphId(0))
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
        for word in text.split_whitespace() {
            match lines.last_mut() {
                Some(last) if self.width(&format!("{last} {word}"), size, spacing) <= max_width => {
                    last.push(' ');
                    last.push_str(word);
                }
                _ => lines.push(word.to_string()),
            }
        }
        lines
    }
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

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    // How far along a corner's edge the curve's control points sit, for a near-perfect arc.
    let k = r * 0.552;
    let mut b = PathBuilder::new();
    b.move_to(x + r, y);
    b.line_to(x + w - r, y);
    b.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    b.line_to(x + w, y + h - r);
    b.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    b.line_to(x + r, y + h);
    b.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    b.line_to(x, y + r);
    b.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    b.close();
    b.finish()
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
            when: "Wed 5 Aug, 14:00",
            dark,
            light,
        }
    }

    #[test]
    fn square_logos_on_a_solid_colour_fill_the_badge_with_it() {
        let solid = |rgba: [u8; 4]| {
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(200, 200, image::Rgba(rgba)))
        };
        let mut black = solid([0, 0, 0, 255]).to_rgba8();
        // Artwork in the middle doesn't matter.
        for (x, y, px) in black.enumerate_pixels_mut() {
            if (50..150).contains(&x) && (50..150).contains(&y) {
                *px = image::Rgba([80, 160, 220, 255]);
            }
        }
        assert_eq!(
            edge_color(&DynamicImage::ImageRgba8(black)),
            Some([0, 0, 0])
        );
        assert_eq!(edge_color(&solid([255, 255, 255, 255])), Some(WHITE));
        assert_eq!(edge_color(&solid([0, 0, 0, 0])), None);

        let mut mixed = solid([0, 0, 0, 255]).to_rgba8();
        for (x, _, px) in mixed.enumerate_pixels_mut() {
            if x >= 100 {
                *px = image::Rgba([255, 0, 0, 255]);
            }
        }
        assert_eq!(edge_color(&DynamicImage::ImageRgba8(mixed)), None);
    }

    #[test]
    fn a_plain_frame_around_a_coloured_logo_is_cut_away() {
        let picture = |background: [u8; 4], middle: [u8; 4]| {
            let mut image = RgbaImage::from_pixel(200, 120, image::Rgba(background));
            for (x, y, px) in image.enumerate_pixels_mut() {
                if (10..190).contains(&x) && (10..110).contains(&y) {
                    *px = image::Rgba(middle);
                }
            }
            DynamicImage::ImageRgba8(image)
        };
        // A yellow rectangle saved on white: the white frame goes, the badge turns yellow.
        let framed = picture([255, 255, 255, 255], [250, 230, 60, 255]);
        let (logo, color) = badge_logo(&framed);
        assert_eq!(
            (logo.width(), logo.height(), color),
            (180, 100, [250, 230, 60])
        );

        // Artwork in the middle of a black square: kept whole, badge black.
        let mut artwork = RgbaImage::from_pixel(200, 200, image::Rgba([0, 0, 0, 255]));
        for (x, y, px) in artwork.enumerate_pixels_mut() {
            if (60..140).contains(&x) && (60..140).contains(&y) {
                *px = image::Rgba([80, 160, 220, 255]);
            }
            if (60..80).contains(&x) && (60..140).contains(&y) {
                *px = image::Rgba([255, 0, 0, 255]);
            }
        }
        let artwork = DynamicImage::ImageRgba8(artwork);
        let (logo, color) = badge_logo(&artwork);
        assert_eq!((logo.width(), color), (200, BLACK));

        // Artwork on a see-through background with a mixed outline: stays on white.
        let mut clear = RgbaImage::from_pixel(200, 200, image::Rgba([0, 0, 0, 0]));
        for (x, y, px) in clear.enumerate_pixels_mut() {
            if (50..150).contains(&x) && (50..150).contains(&y) {
                *px = image::Rgba(if x < 100 {
                    [200, 30, 30, 255]
                } else {
                    [30, 30, 200, 255]
                });
            }
        }
        let (_, color) = badge_logo(&DynamicImage::ImageRgba8(clear));
        assert_eq!(color, WHITE);
    }

    #[test]
    fn initials_use_the_first_two_words() {
        assert_eq!(initials("Townsville Tigersharks B"), "TT");
        assert_eq!(initials("Brisbane"), "BR");
        assert_eq!(initials("  (A) team"), "TE");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn long_names_wrap_onto_two_lines_and_shrink_if_needed() {
        let font = Font::new().unwrap();
        let (size, lines) = font.fit("INDONESIA ELITE WOMEN", NAME_SIZE, 0.0, NAME_MAX_WIDTH, 2);
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
    fn renders_a_youtube_sized_jpeg_without_a_banner_or_logos() {
        let painter = Painter::new("Test Cup", None).unwrap();
        let jpeg = painter
            .render(&game(
                Team {
                    name: "Sydney Kings A",
                    known: true,
                    logo: None,
                },
                Team {
                    name: "Winner G52",
                    known: false,
                    logo: None,
                },
            ))
            .unwrap();
        assert!(jpeg.len() <= MAX_BYTES);
        let image = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((image.width(), image.height()), (WIDTH, HEIGHT));
    }
}
