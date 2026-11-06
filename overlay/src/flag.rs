//! Handles everything to do with rendering flags like the GOAL flag, penalty flags and foul flags.
//! Create an instance of `FlagRenderer` and push Flags into it.
//! Flags are discarded automatically after their 5 second show time as long as the draw function is called.
//!
//! Penalty and foul flags also show a **description bar** with the foul's name ("OBSTRUCTION"),
//! a separate rounded bar just after the name bar. It slides out from left to right, stays for
//! [`DESCRIPTION_HOLD`] seconds and slides back. A foul flag (a foul with no penalty) then fades
//! away; a penalty flag stays for the whole penalty, as before.

use crate::{
    load_images::{Texture, asset_load},
    pages::{Interpolate, Justify, draw_text_both_ex, draw_texture_both_ex, fit_text},
    words::Lang,
};
use macroquad::prelude::*;
use uwh_common::{
    bundles::OptColorBundle,
    color::Color as UWHColor,
    game_snapshot::{Infraction, InfractionSnapshot, PenaltyTime},
};

/// Distance from the top of the screen from where the flags are rendered
const BASE_HEIGHT: f32 = 150f32;

/// Vertical space allocated to each flag
const FLAG_HEIGHT: f32 = 70f32;

/// Left edge of every flag card.
const FLAG_X: f32 = 25f32;

/// How long the card shows on its own before the description bar starts to slide out, in seconds.
const DESCRIPTION_DELAY: f64 = 1.0;

/// How long the description bar stays fully out, in seconds.
const DESCRIPTION_HOLD: f64 = 10.0;

/// How long the description bar takes to slide out, and again to slide back, in seconds.
const DESCRIPTION_SLIDE: f64 = 0.45;

/// How long a foul flag is shown before it fades out: the pause, then the bar's slide out, hold
/// and slide back.
const FOUL_LIFETIME: f64 =
    DESCRIPTION_DELAY + DESCRIPTION_SLIDE + DESCRIPTION_HOLD + DESCRIPTION_SLIDE;

/// The card artwork is 679 px wide. Its first `CARD_LEFT` px (the tab and the start of the name
/// bar) and last `CARD_RIGHT` px (the bar's rounded end) are drawn as they are; the plain strip
/// between them is stretched or shrunk so the card fits the name.
const CARD_ART_WIDTH: f32 = 679f32;
const CARD_LEFT: f32 = 150f32;
const CARD_RIGHT: f32 = 79f32;
/// The standard card width, which fits most names. Longer names make the card grow.
const CARD_MIN_WIDTH: f32 = 480f32;
/// The widest a card gets. A name that still doesn't fit is cut short with "..".
const CARD_MAX_WIDTH: f32 = 1000f32;
// Even the standard card keeps its tab and rounded end whole.
const _: () = assert!(CARD_MIN_WIDTH > CARD_LEFT + CARD_RIGHT);
/// Where the name starts, measured from the card's left edge.
const NAME_X: f32 = 135f32;
/// Space between the end of the name and the card's right end.
const NAME_END_PADDING: f32 = 28f32;
/// The name bar ends 2 px before the artwork does (its shadow); the description bar starts a small
/// gap after that.
const BAR_GAP: f32 = 10f32 - 2f32;
/// Same height as the card's name bar (rows 0-41 of the 48 px artwork).
const BAR_HEIGHT: f32 = 42f32;
/// Corner radius, matching the name bar's rounded right end.
const BAR_RADIUS: f32 = 7f32;
/// Space between the bar's ends and its text.
const BAR_PADDING: f32 = 22f32;
/// Width of the thin red edge on a penalty's description bar.
const BAR_EDGE: f32 = 6f32;
const BAR_FONT_SIZE: u16 = 28;
/// Text baseline, measured from the top of the bar.
const BAR_BASELINE: f32 = 31f32;

/// The foul card's tab and description bar colour.
const FOUL_YELLOW: Color = Color::new(242f32 / 255f32, 183f32 / 255f32, 5f32 / 255f32, 1f32);
/// Text on yellow.
const FOUL_TEXT: Color = Color::new(58f32 / 255f32, 42f32 / 255f32, 0f32, 1f32);
/// The penalty description bar's thin left edge.
const PENALTY_EDGE: Color = Color::new(214f32 / 255f32, 31f32 / 255f32, 31f32 / 255f32, 1f32);
/// The penalty description bar: the same see-through black or white as the card's name bar.
const PENALTY_BAR_ALPHA: f32 = 0.75;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Type {
    Goal(UWHColor, bool),
    /// Third enum value is used to keep track of whether the flag was visited in last sync. Unvisited flags need to be deleted.
    Penalty(UWHColor, PenaltyTime, bool),
    /// A foul with no penalty, against a team (`None`: against neither team).
    Foul(Option<UWHColor>),
}

struct Textures {
    black_goal: Texture,
    white_goal: Texture,
    white_penalty: Texture,
    black_penalty: Texture,
    white_foul: Texture,
    black_foul: Texture,
    font: Font,
}

#[derive(Debug, Clone)]
pub struct Flag {
    player_name: String,
    /// `None` only for a foul the refbox recorded without a player.
    player_number: Option<u8>,
    flag_type: Type,
    /// The foul, shown in the description bar. `Unknown` shows no bar.
    infraction: Infraction,
    /// When the flag first appeared (`get_time()`), which times the description bar.
    shown_at: f64,
    /// Index of the flag's position starting from the top flag.
    vertical_position: u32,
    alpha_animation_counter: f32,
    movement_animation_counter: f32,
}

impl Flag {
    pub const fn new(
        player_name: String,
        player_number: Option<u8>,
        flag_type: Type,
        infraction: Infraction,
    ) -> Self {
        Self {
            player_name,
            player_number,
            flag_type,
            infraction,
            shown_at: 0f64,
            alpha_animation_counter: 0f32,
            vertical_position: 0,
            movement_animation_counter: 0f32,
        }
    }

    fn is_foul_matching(&self, key: &FoulKey) -> bool {
        matches!(self.flag_type, Type::Foul(color) if color == key.0)
            && self.player_number == key.1
            && self.infraction == key.2
    }
}

/// What tells two fouls apart: team, cap number and foul. Identical fouls are counted.
type FoulKey = (Option<UWHColor>, Option<u8>, Infraction);

/// How many of each foul the refbox lists.
fn foul_counts(fouls: &OptColorBundle<Vec<InfractionSnapshot>>) -> Vec<(FoulKey, usize)> {
    let mut counts: Vec<(FoulKey, usize)> = Vec::new();
    for (color, list) in fouls.iter() {
        for foul in list {
            let key = (color, foul.player_number, foul.infraction);
            match counts.iter_mut().find(|(k, _)| *k == key) {
                Some((_, count)) => *count += 1,
                None => counts.push((key, 1)),
            }
        }
    }
    counts
}

fn count_of(counts: &[(FoulKey, usize)], key: &FoulKey) -> usize {
    counts
        .iter()
        .find(|(k, _)| k == key)
        .map_or(0, |(_, count)| *count)
}

/// For every foul whose count changed: how many were added (positive) or removed (negative).
fn foul_changes(before: &[(FoulKey, usize)], after: &[(FoulKey, usize)]) -> Vec<(FoulKey, isize)> {
    let mut changes: Vec<(FoulKey, isize)> = Vec::new();
    for (key, _) in after.iter().chain(before.iter()) {
        if changes.iter().any(|(k, _)| k == key) {
            continue;
        }
        let delta = count_of(after, key) as isize - count_of(before, key) as isize;
        if delta != 0 {
            changes.push((*key, delta));
        }
    }
    changes
}

/// How far out the description bar is (0 = hidden, 1 = fully out), `since_shown` seconds after the
/// flag appeared.
fn description_open(since_shown: f64) -> f32 {
    // Fast at first, gentle at the end, like the mock-up.
    let ease_out = |t: f64| 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3);
    let elapsed = since_shown - DESCRIPTION_DELAY;
    let open = if elapsed <= 0.0 {
        0.0
    } else if elapsed < DESCRIPTION_SLIDE {
        ease_out(elapsed / DESCRIPTION_SLIDE)
    } else if elapsed < DESCRIPTION_SLIDE + DESCRIPTION_HOLD {
        1.0
    } else {
        1.0 - ease_out((elapsed - DESCRIPTION_SLIDE - DESCRIPTION_HOLD) / DESCRIPTION_SLIDE)
    };
    open as f32
}

/// How wide a card is for a name `name_width` px wide: the standard width, or wider for a long
/// name, up to the maximum.
fn card_width(name_width: f32) -> f32 {
    (NAME_X + name_width + NAME_END_PADDING).clamp(CARD_MIN_WIDTH, CARD_MAX_WIDTH)
}

/// Draws the card artwork `width` px wide, stretching only its plain middle strip, into both the
/// colour picture and the transparency picture.
fn draw_card(tex: &Texture, x: f32, y: f32, width: f32, color: Color) {
    let height = tex.color.height();
    let middle = width - CARD_LEFT - CARD_RIGHT;
    let parts = [
        (0f32, CARD_LEFT, x, CARD_LEFT),
        (
            CARD_LEFT,
            CARD_ART_WIDTH - CARD_LEFT - CARD_RIGHT,
            x + CARD_LEFT,
            middle,
        ),
        (
            CARD_ART_WIDTH - CARD_RIGHT,
            CARD_RIGHT,
            x + CARD_LEFT + middle,
            CARD_RIGHT,
        ),
    ];
    for (src_x, src_w, dest_x, dest_w) in parts {
        draw_texture_both_ex!(
            tex,
            dest_x,
            y,
            color,
            DrawTextureParams {
                source: Some(Rect::new(src_x, 0f32, src_w, height)),
                dest_size: Some(vec2(dest_w, height)),
                ..Default::default()
            }
        );
    }
}

/// The description bar's text, in capitals, or `None` when the foul isn't known.
fn description_text(infraction: Infraction, lang: Lang) -> Option<String> {
    lang.infraction(infraction).map(str::to_uppercase)
}

/// The foul card's colour: the penalty card with its red tab turned yellow. Every red pixel,
/// including the soft edge of the tab's curve, is blended towards yellow by how red it is,
/// keeping the tab's shading. Other pixels (the name bar, the shadow) are left as they are.
fn tab_red_to_yellow([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let redness = i32::from(r) - i32::from(g.max(b));
    if redness <= 40 {
        return [r, g, b, a];
    }
    let k = (redness as f32 / 170f32).min(1f32);
    let shade = 0.45f32.mul_add(f32::from(r) / 255f32, 0.55);
    let mix = |from: u8, to: f32| f32::from(from).mul_add(1f32 - k, to * k).round() as u8;
    [
        mix(r, 242f32 * shade),
        mix(g, 183f32 * shade),
        mix(b, 5f32),
        a,
    ]
}

/// The foul card artwork, made from the penalty card's when the overlay starts. The shape is the
/// same, so the penalty card's transparency mask is used as it is.
fn foul_texture(penalty_color: &[u8], penalty_alpha: &[u8]) -> Texture {
    // The artwork is built into the program, so it can't fail to load; the penalty cards load
    // the same bytes with `Texture2D::from_file_with_format`, which panics the same way.
    let mut image = Image::from_file_with_format(penalty_color, None)
        .expect("built-in penalty card artwork should load");
    for pixel in image.get_image_data_mut() {
        *pixel = tab_red_to_yellow(*pixel);
    }
    Texture {
        color: Texture2D::from_image(&image),
        alpha: Texture2D::from_file_with_format(penalty_alpha, None),
    }
}

/// Fills a rectangle whose left and/or right corners are rounded (radius 0: square).
fn fill_rounded_rect(x: f32, y: f32, w: f32, h: f32, r_left: f32, r_right: f32, color: Color) {
    const STEPS: usize = 8;
    if w <= 0f32 || h <= 0f32 {
        return;
    }
    let r_left = r_left.min(h / 2f32).min(w);
    let r_right = r_right.min(h / 2f32).min(w - r_left);
    let mut points: Vec<Vec2> = Vec::with_capacity(4 * (STEPS + 1));
    let mut arc = |cx: f32, cy: f32, r: f32, from_deg: f32| {
        for step in 0..=STEPS {
            let angle = (90f32 * step as f32 / STEPS as f32 + from_deg).to_radians();
            points.push(vec2(r.mul_add(angle.cos(), cx), r.mul_add(angle.sin(), cy)));
        }
    };
    // Clockwise on screen (y down), starting at the top-left corner.
    arc(x + r_left, y + r_left, r_left, 180f32);
    arc(x + w - r_right, y + r_right, r_right, 270f32);
    arc(x + w - r_right, y + h - r_right, r_right, 0f32);
    arc(x + r_left, y + h - r_left, r_left, 90f32);
    let centre = vec2(x + w / 2f32, y + h / 2f32);
    for i in 0..points.len() {
        let next = points[(i + 1) % points.len()];
        draw_triangle(centre, points[i], next, color);
    }
}

pub struct Renderer {
    active_flags: Vec<Flag>,
    inactive_flags: Vec<Flag>,
    textures: Textures,
    /// The fouls listed in the last snapshot. `None` until the first snapshot arrives, which only
    /// sets the starting point: fouls already on the refbox when the overlay starts aren't shown.
    seen_fouls: Option<Vec<(FoulKey, usize)>>,
}

impl Renderer {
    pub fn add_flag(&mut self, mut flag: Flag, game_state: &crate::State) {
        flag.vertical_position = self.active_flags.len() as u32;
        flag.shown_at = get_time();
        let team = match flag.flag_type {
            Type::Goal(color, _) | Type::Penalty(color, _, _) => Some(color),
            Type::Foul(color) => color,
        };
        let team_info = |color| match color {
            UWHColor::Black => &game_state.black,
            UWHColor::White => &game_state.white,
        };
        flag.player_name = match (team, flag.player_number) {
            (Some(color), Some(number)) => team_info(color)
                .get_players()
                .find(|player| player.number.unwrap() == number)
                .map(|player| player.name.clone())
                .unwrap_or_default(),
            (Some(color), None) => team_info(color).team_name.clone(),
            // Drawn as "Both teams" in the overlay's language (`draw_flag`).
            (None, _) => String::new(),
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
                white_foul: foul_texture(
                    include_bytes!("../assets/color/1080/Penalty White.png"),
                    include_bytes!("../assets/alpha/1080/Penalty White.png"),
                ),
                black_foul: foul_texture(
                    include_bytes!("../assets/color/1080/Penalty Black.png"),
                    include_bytes!("../assets/alpha/1080/Penalty Black.png"),
                ),
                font: load_ttf_font_from_bytes(include_bytes!("./../assets/BAHNSCHRIFT.TTF"))
                    .unwrap(),
            },
            seen_fouls: None,
        }
    }

    pub fn synchronize_flags(&mut self, game_state: &crate::State) {
        self.synchronize_goals(game_state);
        self.synchronize_penalties(UWHColor::Black, game_state);
        self.synchronize_penalties(UWHColor::White, game_state);
        self.synchronize_fouls(game_state);
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
                        && flag.player_number == Some(goal.1)
                })
                .unwrap_or_else(|| {
                    self.add_flag(
                        Flag::new(
                            String::new(),
                            Some(goal.1),
                            Type::Goal(goal.0, true),
                            Infraction::Unknown,
                        ),
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
        for penalty in &game_state.snapshot.penalties[team_color] {
            if !matches!(penalty.time, PenaltyTime::Seconds(0)) {
                // find the penalty in the local list, create a new penalty if it doesn't exist.
                let flag_pos = self
                    .active_flags
                    .iter()
                    .position(|flag| {
                        matches!(flag.flag_type, Type::Penalty(color, _, _) if color == team_color )
                            && flag.player_number == Some(penalty.player_number)
                    })
                    .unwrap_or_else(|| {
                        self.add_flag(
                            Flag::new(
                                String::new(),
                                Some(penalty.player_number),
                                Type::Penalty(team_color, penalty.time, true),
                                penalty.infraction,
                            ),
                            game_state,
                        );
                        self.active_flags.len() - 1
                    });

                // update time on all the penalty flags
                match self.active_flags.get_mut(flag_pos).unwrap() {
                    Flag {
                        flag_type: Type::Penalty(_, time, is_visited),
                        infraction,
                        ..
                    } => {
                        *time = penalty.time;
                        *is_visited = true;
                        // A foul corrected on the refbox while the bar is out shows the correction.
                        *infraction = penalty.infraction;
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
        self.active_flags.sort_by(flag_order);
    }

    /// Shows a foul flag for each foul added on the refbox, and takes one down for each foul
    /// deleted there. Fouls are told apart by team, cap number and foul, and identical ones are
    /// counted, since the refbox's list carries no ids.
    fn synchronize_fouls(&mut self, game_state: &crate::State) {
        let counts = foul_counts(&game_state.snapshot.fouls);
        let Some(before) = self.seen_fouls.replace(counts.clone()) else {
            return;
        };
        for (key, delta) in foul_changes(&before, &counts) {
            if delta > 0 {
                for _ in 0..delta {
                    self.add_flag(
                        Flag::new(String::new(), key.1, Type::Foul(key.0), key.2),
                        game_state,
                    );
                }
            } else {
                for _ in 0..-delta {
                    // The newest matching flag still on screen, if any (it may have timed out).
                    if let Some(pos) = self
                        .active_flags
                        .iter()
                        .rposition(|flag| flag.is_foul_matching(&key))
                    {
                        let flag = self.active_flags.remove(pos);
                        self.inactive_flags.push(flag);
                    }
                }
            }
        }
        self.active_flags.sort_by(flag_order);
    }

    /// Responsible for drawing the flags, deleting them, etc.
    pub fn draw(&mut self, lang: Lang) {
        let now = get_time();
        // Foul flags whose time is up fade out.
        let (expired, kept): (Vec<Flag>, Vec<Flag>) = std::mem::take(&mut self.active_flags)
            .into_iter()
            .partition(|flag| {
                matches!(flag.flag_type, Type::Foul(_)) && now - flag.shown_at >= FOUL_LIFETIME
            });
        self.active_flags = kept;
        self.inactive_flags.extend(expired);

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
            draw_flag(
                &self.textures,
                flag,
                (idx as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT) + movement_offset,
                alpha_offset,
                now,
                lang,
            );
        }
        for flag in &mut self.inactive_flags {
            flag.alpha_animation_counter -= 1f32 / 60f32;
            let alpha_offset = (0f32, 1f32).interpolate_linear(flag.alpha_animation_counter);
            draw_flag(
                &self.textures,
                flag,
                (flag.vertical_position as f32).mul_add(FLAG_HEIGHT, BASE_HEIGHT),
                alpha_offset,
                now,
                lang,
            );
        }
        // delete flags marked as unvisited and that have their alpha_animation_counter below zero (finihed fade out)
        self.inactive_flags
            .retain(|x| x.alpha_animation_counter > 0f32);
    }
}

/// The order of the flags from the top: TD penalties, then time penalties by longest time, then
/// fouls, then goal callouts. Flags of the same kind keep their order.
fn flag_order(a: &Flag, b: &Flag) -> std::cmp::Ordering {
    let rank = |flag: &Flag| match flag.flag_type {
        Type::Penalty(..) => 0,
        Type::Foul(_) => 1,
        Type::Goal(..) => 2,
    };
    if let (Type::Penalty(_, time_a, _), Type::Penalty(_, time_b, _)) = (&a.flag_type, &b.flag_type)
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
    } else {
        rank(a).cmp(&rank(b))
    }
}

/// Draws one flag (card, name, tab text and description bar) with its top at `y`.
fn draw_flag(textures: &Textures, flag: &Flag, y: f32, alpha: f32, now: f64, lang: Lang) {
    let (tex, dark_card) = match flag.flag_type {
        Type::Goal(color, _) => (
            if color == UWHColor::White {
                &textures.white_goal
            } else {
                &textures.black_goal
            },
            color == UWHColor::Black,
        ),
        Type::Penalty(color, _, _) => (
            if color == UWHColor::White {
                &textures.white_penalty
            } else {
                &textures.black_penalty
            },
            color == UWHColor::Black,
        ),
        Type::Foul(color) => (
            if color == Some(UWHColor::Black) {
                &textures.black_foul
            } else {
                &textures.white_foul
            },
            color == Some(UWHColor::Black),
        ),
    };
    let card_text_color = Color {
        a: alpha,
        ..if dark_card { WHITE } else { BLACK }
    };
    let name = match (flag.player_number, &flag.flag_type) {
        (Some(number), _) => format!("#{} {}", number, flag.player_name),
        (None, Type::Foul(None)) => lang.both_teams().to_string(),
        (None, _) => flag.player_name.clone(),
    };
    let (_, name) = fit_text(
        CARD_MAX_WIDTH - NAME_X - NAME_END_PADDING,
        &name,
        30,
        &textures.font,
        Justify::Left,
    );
    let width = card_width(measure_text(&name, Some(&textures.font), 30, 1f32).width);
    draw_card(tex, FLAG_X, y, width, Color { a: alpha, ..WHITE });
    draw_text_both_ex!(
        name.as_str(),
        FLAG_X + NAME_X,
        y + 33f32,
        TextParams {
            font: Some(&textures.font),
            font_size: 30,
            color: card_text_color,
            ..Default::default()
        },
        TextParams {
            font: Some(&textures.font),
            font_size: 30,
            color: Color { a: alpha, ..WHITE },
            ..Default::default()
        }
    );
    let tab_text_params = |color: Color| TextParams {
        font: Some(&textures.font),
        font_size: 30,
        color,
        ..Default::default()
    };
    match flag.flag_type {
        Type::Goal(_, _) => {
            let (x_off, text) = fit_text(94f32, lang.goal(), 30, &textures.font, Justify::Center);
            draw_text_ex(
                text.as_str(),
                35f32 + x_off,
                y + 33f32,
                tab_text_params(card_text_color),
            );
        }
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
            let (x_off, text) = fit_text(94f32, &text, 30, &textures.font, Justify::Center);
            // White on the red tab for both teams: black is hard to read on its darker red.
            draw_text_ex(
                text.as_str(),
                35f32 + x_off,
                y + 33f32,
                tab_text_params(Color { a: alpha, ..WHITE }),
            );
        }
        Type::Foul(_) => {
            let (x_off, text) = fit_text(94f32, lang.foul(), 30, &textures.font, Justify::Center);
            draw_text_ex(
                text.as_str(),
                35f32 + x_off,
                y + 33f32,
                tab_text_params(Color {
                    a: alpha,
                    ..FOUL_TEXT
                }),
            );
        }
    }
    if matches!(flag.flag_type, Type::Goal(..)) {
        return;
    }
    if let Some(text) = description_text(flag.infraction, lang) {
        let open = description_open(now - flag.shown_at);
        let style = match flag.flag_type {
            Type::Foul(_) => BarStyle::Foul,
            _ if dark_card => BarStyle::DarkPenalty,
            _ => BarStyle::LightPenalty,
        };
        let bar_x = FLAG_X + width + BAR_GAP;
        draw_description(textures, &text, style, bar_x, y, alpha, open);
    }
}

/// The description bar's look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BarStyle {
    /// Yellow, with dark text.
    Foul,
    /// See-through black with a red edge and white text (Black team penalty).
    DarkPenalty,
    /// See-through white with a red edge and black text (White team penalty).
    LightPenalty,
}

impl BarStyle {
    /// The bar's body colour and its text colour.
    const fn colours(self) -> (Color, Color) {
        match self {
            Self::Foul => (FOUL_YELLOW, FOUL_TEXT),
            Self::DarkPenalty => (BLACK, WHITE),
            Self::LightPenalty => (WHITE, BLACK),
        }
    }
}

/// Draws the description bar `open` of the way out (0 to 1), into both the colour picture and
/// the transparency picture, so it keys cleanly.
fn draw_description(
    textures: &Textures,
    text: &str,
    style: BarStyle,
    bar_x: f32,
    y: f32,
    alpha: f32,
    open: f32,
) {
    if open <= 0f32 || alpha <= 0f32 {
        return;
    }
    let full_width = 2f32.mul_add(
        BAR_PADDING,
        measure_text(text, Some(&textures.font), BAR_FONT_SIZE, 1f32).width,
    );
    let width = full_width * open.min(1f32);
    let key = |a: f32| Color {
        a: a * alpha,
        ..WHITE
    };
    let (body, text_color) = style.colours();
    for (x, colour_half) in [(bar_x, true), (bar_x + 1920f32, false)] {
        if style == BarStyle::Foul {
            let color = if colour_half {
                Color { a: alpha, ..body }
            } else {
                key(1f32)
            };
            fill_rounded_rect(x, y, width, BAR_HEIGHT, BAR_RADIUS, BAR_RADIUS, color);
        } else {
            let edge = BAR_EDGE.min(width);
            let (edge_color, body_color) = if colour_half {
                (
                    Color {
                        a: alpha,
                        ..PENALTY_EDGE
                    },
                    Color {
                        a: PENALTY_BAR_ALPHA * alpha,
                        ..body
                    },
                )
            } else {
                (key(1f32), key(PENALTY_BAR_ALPHA))
            };
            fill_rounded_rect(x, y, edge, BAR_HEIGHT, BAR_RADIUS, 0f32, edge_color);
            fill_rounded_rect(
                x + edge,
                y,
                width - edge,
                BAR_HEIGHT,
                0f32,
                BAR_RADIUS,
                body_color,
            );
        }
    }

    // While the bar slides, show only the letters it already covers.
    let room = width - BAR_PADDING;
    let mut shown = String::new();
    for c in text.chars() {
        shown.push(c);
        if measure_text(&shown, Some(&textures.font), BAR_FONT_SIZE, 1f32).width > room {
            shown.pop();
            break;
        }
    }
    if shown.is_empty() {
        return;
    }
    draw_text_both_ex!(
        shown.as_str(),
        bar_x + BAR_PADDING,
        y + BAR_BASELINE,
        TextParams {
            font: Some(&textures.font),
            font_size: BAR_FONT_SIZE,
            color: Color {
                a: alpha,
                ..text_color
            },
            ..Default::default()
        },
        TextParams {
            font: Some(&textures.font),
            font_size: BAR_FONT_SIZE,
            color: Color { a: alpha, ..WHITE },
            ..Default::default()
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foul(number: Option<u8>, infraction: Infraction) -> InfractionSnapshot {
        InfractionSnapshot {
            player_number: number,
            infraction,
        }
    }

    #[test]
    fn a_new_foul_is_one_change_and_an_unchanged_list_is_none() {
        let mut fouls = OptColorBundle::<Vec<InfractionSnapshot>>::default();
        fouls
            .white
            .push(foul(Some(4), Infraction::StickInfringement));
        let before = foul_counts(&fouls);
        assert!(foul_changes(&before, &before).is_empty());

        fouls.black.push(foul(Some(9), Infraction::Obstruction));
        let after = foul_counts(&fouls);
        assert_eq!(
            foul_changes(&before, &after),
            vec![((Some(UWHColor::Black), Some(9), Infraction::Obstruction), 1)]
        );
    }

    #[test]
    fn identical_fouls_are_counted_and_a_deleted_one_is_a_removal() {
        let mut fouls = OptColorBundle::<Vec<InfractionSnapshot>>::default();
        fouls.white.push(foul(Some(4), Infraction::FreeArm));
        let one = foul_counts(&fouls);
        fouls.white.push(foul(Some(4), Infraction::FreeArm));
        let two = foul_counts(&fouls);
        let key = (Some(UWHColor::White), Some(4), Infraction::FreeArm);
        assert_eq!(foul_changes(&one, &two), vec![(key, 1)]);
        assert_eq!(foul_changes(&two, &one), vec![(key, -1)]);
        assert_eq!(foul_changes(&one, &[]), vec![(key, -1)]);
    }

    #[test]
    fn a_foul_against_neither_team_has_no_colour() {
        let mut fouls = OptColorBundle::<Vec<InfractionSnapshot>>::default();
        fouls.equal.push(foul(None, Infraction::DelayOfGame));
        assert_eq!(
            foul_counts(&fouls),
            vec![((None, None, Infraction::DelayOfGame), 1)]
        );
    }

    #[test]
    fn the_bar_waits_a_second_slides_out_holds_ten_seconds_and_slides_back() {
        assert_eq!(description_open(0.0), 0.0);
        assert_eq!(description_open(0.9), 0.0, "the card shows alone for 1 s");
        assert!(description_open(1.2) > 0.0 && description_open(1.2) < 1.0);
        assert_eq!(description_open(1.0 + DESCRIPTION_SLIDE), 1.0);
        assert_eq!(description_open(1.0 + DESCRIPTION_SLIDE + 9.9), 1.0);
        let closing = description_open(1.0 + DESCRIPTION_SLIDE + DESCRIPTION_HOLD + 0.2);
        assert!(closing > 0.0 && closing < 1.0);
        assert_eq!(description_open(FOUL_LIFETIME), 0.0);
        assert_eq!(description_open(FOUL_LIFETIME + 60.0), 0.0);
        assert!((FOUL_LIFETIME - 11.9).abs() < 1e-9);
    }

    #[test]
    fn a_card_is_standard_width_and_grows_for_long_names_up_to_a_limit() {
        assert_eq!(card_width(0f32), CARD_MIN_WIDTH);
        assert_eq!(
            card_width(150f32),
            CARD_MIN_WIDTH,
            "a short name keeps the standard width"
        );
        assert_eq!(card_width(600f32), NAME_X + 600f32 + NAME_END_PADDING);
        assert_eq!(card_width(5000f32), CARD_MAX_WIDTH);
    }

    #[test]
    fn a_white_team_penalty_bar_is_white_with_black_text() {
        assert_eq!(BarStyle::LightPenalty.colours(), (WHITE, BLACK));
        assert_eq!(BarStyle::DarkPenalty.colours(), (BLACK, WHITE));
        assert_eq!(BarStyle::Foul.colours(), (FOUL_YELLOW, FOUL_TEXT));
    }

    #[test]
    fn the_description_is_in_capitals_and_unknown_has_none() {
        assert_eq!(
            description_text(Infraction::StickInfringement, Lang::En).as_deref(),
            Some("STICK FOUL")
        );
        assert_eq!(description_text(Infraction::Unknown, Lang::En), None);
        assert_eq!(
            description_text(Infraction::FalseStart, Lang::Fr).as_deref(),
            Some("FAUX DÉPART")
        );
    }

    #[test]
    fn only_red_turns_yellow() {
        // The tab's red, top and bottom of its shading.
        let top = tab_red_to_yellow([232, 0, 0, 255]);
        assert!(top[0] > 200 && top[1] > 150 && top[2] < 20 && top[3] == 255);
        let bottom = tab_red_to_yellow([79, 0, 0, 255]);
        assert!(bottom[1] > 0, "even the darkest red gets some yellow");
        // The soft edge keeps its transparency.
        assert_eq!(tab_red_to_yellow([167, 0, 0, 230])[3], 230);
        // The name bar and the shadow are untouched.
        for pixel in [[255, 255, 255, 191], [0, 0, 0, 191], [0, 0, 0, 0]] {
            assert_eq!(tab_red_to_yellow(pixel), pixel);
        }
    }

    #[test]
    fn fouls_sit_between_penalties_and_goals() {
        let penalty = Flag::new(
            String::new(),
            Some(1),
            Type::Penalty(UWHColor::Black, PenaltyTime::Seconds(30), true),
            Infraction::Obstruction,
        );
        let td = Flag::new(
            String::new(),
            Some(2),
            Type::Penalty(UWHColor::White, PenaltyTime::TotalDismissal, true),
            Infraction::Unknown,
        );
        let foul = Flag::new(
            String::new(),
            Some(3),
            Type::Foul(Some(UWHColor::White)),
            Infraction::FreeArm,
        );
        let goal = Flag::new(
            String::new(),
            Some(4),
            Type::Goal(UWHColor::White, true),
            Infraction::Unknown,
        );
        let mut flags = [goal, foul, penalty, td];
        flags.sort_by(flag_order);
        let order: Vec<Option<u8>> = flags.iter().map(|f| f.player_number).collect();
        assert_eq!(order, vec![Some(2), Some(1), Some(3), Some(4)]);
    }
}
