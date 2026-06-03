use iced::{
    Color, Font, Point, Size,
    alignment::{Horizontal, Vertical},
    widget::canvas::{Fill, Frame},
};
use iced_core::text::{LineHeight, Shaping};
use iced_graphics::geometry::Text;
use matrix_drawing::{secs_to_time_string, transmitted_data::TransmittedData};
use uwh_common::game_snapshot::TimeoutSnapshot;

const BG: Color = Color::BLACK;
const CLOCK_YELLOW: Color = Color::from_rgb(1.0, 1.0, 0.0);
const WHITE_BOX: Color = Color::WHITE;
const BLACK_BOX: Color = Color::from_rgb(0.05, 0.05, 0.05);
const BLACK_BOX_OUTLINE: Color = Color::from_rgb(0.5, 0.5, 0.5);
/// The centre game-state text: a timeout type label during a timeout, otherwise
/// the current period name (uppercased). Always rendered in yellow by callers
/// (the new layouts use a single yellow throughout; the Default panel keeps its
/// own colours unchanged).
fn badge_text(data: &TransmittedData) -> String {
    match data.snapshot.timeout {
        Some(TimeoutSnapshot::White(_)) => "WHITE T/O".to_string(),
        Some(TimeoutSnapshot::Black(_)) => "BLACK T/O".to_string(),
        Some(TimeoutSnapshot::Ref(_)) => "REF T/O".to_string(),
        Some(TimeoutSnapshot::PenaltyShot(_)) => "PENALTY SHOT".to_string(),
        None => data.snapshot.current_period.to_string().to_uppercase(),
    }
}

/// The clock string: the timeout countdown during a timeout, otherwise the
/// remaining seconds in the period.
fn clock_string(data: &TransmittedData) -> String {
    let secs = match data.snapshot.timeout {
        Some(TimeoutSnapshot::White(s))
        | Some(TimeoutSnapshot::Black(s))
        | Some(TimeoutSnapshot::Ref(s))
        | Some(TimeoutSnapshot::PenaltyShot(s)) => s,
        None => data.snapshot.secs_in_period,
    };
    secs_to_time_string(secs).trim().to_string()
}

/// Build a canvas `Text` element, mirroring the construction proven to work in
/// `sunlight_display`. Centred vertically; horizontal alignment is the caller's
/// choice.
fn label(content: String, x: f32, y: f32, size: f32, color: Color, h: Horizontal) -> Text {
    Text {
        content,
        position: Point::new(x, y),
        color,
        size: size.into(),
        line_height: LineHeight::Relative(1.0),
        font: Font::with_name("Roboto"),
        horizontal_alignment: h,
        vertical_alignment: Vertical::Center,
        shaping: Shaping::Basic,
    }
}

/// Draw one team block: a label centred above a filled score box with a large
/// number inside. The BLACK box gets a thin outline so it reads against the
/// black background. Shared by Classic / Corners / ScoresOnly.
// Each parameter is an independent layout dimension with no natural grouping, so
// a params struct would add noise without improving clarity.
#[allow(clippy::too_many_arguments)]
fn draw_team_block(
    frame: &mut Frame,
    x: f32,
    y: f32,
    box_w: f32,
    box_h: f32,
    label_gap: f32,
    lbl: &str,
    score: u8,
    is_white: bool,
    label_size: f32,
) {
    if label_size > 0.0 {
        frame.fill_text(label(
            lbl.to_string(),
            x + box_w / 2.0,
            y - label_gap,
            label_size,
            Color::WHITE,
            Horizontal::Center,
        ));
    }

    let fill = if is_white { WHITE_BOX } else { BLACK_BOX };
    frame.fill_rectangle(Point::new(x, y), Size::new(box_w, box_h), Fill::from(fill));

    if !is_white {
        // Outline thickness scales with the box so it reads at any window size.
        let t = (box_h * 0.02).max(2.0);
        frame.fill_rectangle(
            Point::new(x, y),
            Size::new(box_w, t),
            Fill::from(BLACK_BOX_OUTLINE),
        );
        frame.fill_rectangle(
            Point::new(x, y + box_h - t),
            Size::new(box_w, t),
            Fill::from(BLACK_BOX_OUTLINE),
        );
        frame.fill_rectangle(
            Point::new(x, y),
            Size::new(t, box_h),
            Fill::from(BLACK_BOX_OUTLINE),
        );
        frame.fill_rectangle(
            Point::new(x + box_w - t, y),
            Size::new(t, box_h),
            Fill::from(BLACK_BOX_OUTLINE),
        );
    }

    let score_color = if is_white { Color::BLACK } else { Color::WHITE };
    // Size so a two-digit score (10-99) always fits the fixed box width; single
    // digits render at the same size, centred. The width cap (0.68 * box_w)
    // leaves margin for two Roboto digits inside the box.
    let score_size = (box_h * 0.8).min(box_w * 0.68);
    frame.fill_text(label(
        score.to_string(),
        x + box_w / 2.0,
        y + box_h / 2.0,
        score_size,
        score_color,
        Horizontal::Center,
    ));
}

/// Draw the centre game-state label (period name, or timeout type) as borderless
/// yellow text — no filled pill — matching the Big Time layout. The font shrinks
/// if the text would exceed `max_w` (the room before the score boxes).
fn draw_badge(
    frame: &mut Frame,
    cx: f32,
    cy: f32,
    max_w: f32,
    badge_h: f32,
    data: &TransmittedData,
) {
    const CHAR_W: f32 = 0.66; // approx Roboto uppercase advance per char, in em
    let text = badge_text(data);
    let chars = text.chars().count() as f32;

    let mut font = badge_h * 0.6;
    let needed = chars * font * CHAR_W;
    if needed > max_w {
        font *= max_w / needed;
    }

    frame.fill_text(label(text, cx, cy, font, CLOCK_YELLOW, Horizontal::Center));
}

pub fn draw_classic(frame: &mut Frame, bounds: Size, data: &TransmittedData) {
    let (w, h) = (bounds.width, bounds.height);
    frame.fill_rectangle(Point::ORIGIN, bounds, Fill::from(BG));

    let white_left = !data.white_on_right;
    let box_w = w * 0.22;
    let box_h = h * 0.55;
    let box_y = h * 0.28;
    let left_x = w * 0.04;
    let right_x = w - box_w - w * 0.04;
    let label_size = h * 0.07;
    let label_gap = h * 0.06;

    if white_left {
        draw_team_block(
            frame,
            left_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "WHITE",
            data.snapshot.scores.white,
            true,
            label_size,
        );
        draw_team_block(
            frame,
            right_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "BLACK",
            data.snapshot.scores.black,
            false,
            label_size,
        );
    } else {
        draw_team_block(
            frame,
            left_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "BLACK",
            data.snapshot.scores.black,
            false,
            label_size,
        );
        draw_team_block(
            frame,
            right_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "WHITE",
            data.snapshot.scores.white,
            true,
            label_size,
        );
    }

    let cx = w / 2.0;
    let badge_h = h * 0.12;
    // Room between the two score boxes for the game-state text.
    let badge_max_w = w * 0.46;
    draw_badge(
        frame,
        cx,
        box_y - h * 0.02 + badge_h / 2.0,
        badge_max_w,
        badge_h,
        data,
    );
    // Cap the clock by width so it never overruns into the score boxes at wide /
    // full-screen aspect ratios (height alone makes it too wide there).
    let clock_size = (h * 0.34).min(w * 0.16);
    frame.fill_text(label(
        clock_string(data),
        cx,
        h * 0.62,
        clock_size,
        CLOCK_YELLOW,
        Horizontal::Center,
    ));
}

pub fn draw_big_time(frame: &mut Frame, bounds: Size, data: &TransmittedData) {
    let (w, h) = (bounds.width, bounds.height);
    frame.fill_rectangle(Point::ORIGIN, bounds, Fill::from(BG));

    // Always show the game state across the top: the period name, or the
    // timeout type ("WHITE T/O", "PENALTY SHOT", …) while a timeout runs.
    frame.fill_text(label(
        badge_text(data),
        w / 2.0,
        h * 0.12,
        h * 0.09,
        CLOCK_YELLOW,
        Horizontal::Center,
    ));

    frame.fill_text(label(
        clock_string(data),
        w / 2.0,
        h * 0.56,
        h * 0.6,
        CLOCK_YELLOW,
        Horizontal::Center,
    ));
}

pub fn draw_corners(frame: &mut Frame, bounds: Size, data: &TransmittedData) {
    let (w, h) = (bounds.width, bounds.height);
    frame.fill_rectangle(Point::ORIGIN, bounds, Fill::from(BG));

    let white_left = !data.white_on_right;
    let box_w = w * 0.14;
    let box_h = h * 0.30;
    // Pushed down from the very top so the team labels above the boxes are not
    // clipped off the top edge.
    let box_y = h * 0.13;
    let left_x = w * 0.03;
    let right_x = w - box_w - w * 0.03;
    let label_size = h * 0.06;
    let label_gap = h * 0.06;

    if white_left {
        draw_team_block(
            frame,
            left_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "WHITE",
            data.snapshot.scores.white,
            true,
            label_size,
        );
        draw_team_block(
            frame,
            right_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "BLACK",
            data.snapshot.scores.black,
            false,
            label_size,
        );
    } else {
        draw_team_block(
            frame,
            left_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "BLACK",
            data.snapshot.scores.black,
            false,
            label_size,
        );
        draw_team_block(
            frame,
            right_x,
            box_y,
            box_w,
            box_h,
            label_gap,
            "WHITE",
            data.snapshot.scores.white,
            true,
            label_size,
        );
    }

    let cx = w / 2.0;
    let badge_h = h * 0.12;
    // Corner score boxes sit at the edges, so the centre has generous room.
    let badge_max_w = w * 0.60;
    draw_badge(frame, cx, h * 0.30, badge_max_w, badge_h, data);
    frame.fill_text(label(
        clock_string(data),
        cx,
        h * 0.66,
        h * 0.5,
        CLOCK_YELLOW,
        Horizontal::Center,
    ));
}

pub fn draw_scores_only(frame: &mut Frame, bounds: Size, data: &TransmittedData) {
    let (w, h) = (bounds.width, bounds.height);
    frame.fill_rectangle(Point::ORIGIN, bounds, Fill::from(BG));

    let white_left = !data.white_on_right;
    // Large fixed-size boxes (room for two digits) that fill most of the screen,
    // with a margin from the edges and a gap between them.
    let box_w = w * 0.42;
    let box_h = h * 0.78;
    let box_y = (h - box_h) / 2.0;
    let left_x = w * 0.05;
    let right_x = w - box_w - w * 0.05;

    if white_left {
        draw_team_block(
            frame,
            left_x,
            box_y,
            box_w,
            box_h,
            0.0,
            "WHITE",
            data.snapshot.scores.white,
            true,
            0.0,
        );
        draw_team_block(
            frame,
            right_x,
            box_y,
            box_w,
            box_h,
            0.0,
            "BLACK",
            data.snapshot.scores.black,
            false,
            0.0,
        );
    } else {
        draw_team_block(
            frame,
            left_x,
            box_y,
            box_w,
            box_h,
            0.0,
            "BLACK",
            data.snapshot.scores.black,
            false,
            0.0,
        );
        draw_team_block(
            frame,
            right_x,
            box_y,
            box_w,
            box_h,
            0.0,
            "WHITE",
            data.snapshot.scores.white,
            true,
            0.0,
        );
    }
}
