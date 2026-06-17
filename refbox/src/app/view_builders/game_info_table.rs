// Task 2 of a staged implementation: types and builder are consumed by Tasks 3–8.
// The dead_code allow is removed once the renderer + wiring tasks wire this in.
#![allow(dead_code)]
use super::*;
use iced::{
    Alignment, Element, Length,
    alignment::Horizontal,
    widget::{column, container, row, text},
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::schedule::{Schedule, TeamList},
};

const TEAM_NAME_LEN_LIMIT: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum Variant {
    Full,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum GameRole {
    Last,
    Current,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct TeamLine {
    pub name: Option<String>,
    pub score: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum Row {
    GameBlock {
        role: GameRole,
        number: String,
        game_block: Option<String>,
        white: TeamLine,
        black: TeamLine,
    },
    SettingPair {
        left: (String, String),
        right: Option<(String, String)>,
    },
    Referee {
        label: String,
        name: String,
    },
}

pub(in super::super) fn game_info_rows(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
    last_game_scores: Option<BlackWhiteBundle<u8>>,
    variant: Variant,
) -> Vec<Row> {
    let between = snapshot.current_period == GamePeriod::BetweenGames;
    // The "current" game whose config + settings are displayed: the in-progress
    // game when playing, the upcoming game between games. Matches details_strings.
    let current_game_num: &GameNumber = if between {
        &snapshot.next_game_number
    } else {
        &snapshot.game_number
    };

    let mut rows = Vec::new();

    // --- Current game block ---
    rows.push(game_block_row(
        GameRole::Current,
        current_game_num,
        Some(time_string(config.game_block)),
        Some(snapshot.scores),
        using_uwhportal,
        schedule,
        teams,
    ));

    // --- Settings grid (belongs to the current game) ---
    let mut settings: Vec<(String, String)> = Vec::new();
    if config.single_half {
        settings.push((
            fl!("gi-game-length"),
            time_string(config.half_play_duration),
        ));
    } else {
        settings.push((
            fl!("gi-half-length"),
            time_string(config.half_play_duration),
        ));
        settings.push((
            fl!("gi-half-time-length"),
            time_string(config.half_time_duration),
        ));
    }
    settings.push((fl!("gi-timeouts"), team_timeouts_value(config)));
    if config.num_team_timeouts_allowed != 0 {
        settings.push((
            fl!("gi-timeout-duration"),
            time_string(config.team_timeout_duration),
        ));
    }
    settings.push((fl!("gi-overtime"), bool_string(config.overtime_allowed)));
    settings.push((
        fl!("gi-sudden-death"),
        bool_string(config.sudden_death_allowed),
    ));
    if config.overtime_allowed {
        settings.push((
            fl!("gi-pre-overtime-break"),
            time_string(config.pre_overtime_break),
        ));
    }
    if config.sudden_death_allowed {
        settings.push((
            fl!("gi-pre-sudden-death-break"),
            time_string(config.pre_sudden_death_duration),
        ));
    }
    if config.overtime_allowed {
        settings.push((
            fl!("gi-overtime-half-length"),
            time_string(config.ot_half_play_duration),
        ));
    }
    settings.push((
        fl!("gi-minimum-game-break"),
        time_string(config.minimum_break),
    ));
    if config.overtime_allowed {
        settings.push((
            fl!("gi-overtime-half-time-length"),
            time_string(config.ot_half_time_duration),
        ));
    }
    settings.push((
        fl!("gi-stop-clock-last-2"),
        stop_clock_value(schedule, current_game_num),
    ));

    let mut iter = settings.into_iter();
    while let Some(left) = iter.next() {
        let right = iter.next();
        rows.push(Row::SettingPair { left, right });
    }

    // Context block BEFORE the current block, between games only: the just-finished game.
    if between {
        let last = game_block_row(
            GameRole::Last,
            &snapshot.game_number,
            None, // prior game's Game Block is intentionally not shown
            last_game_scores,
            using_uwhportal,
            schedule,
            teams,
        );
        rows.insert(0, last);
    }

    if using_uwhportal {
        rows.extend(referee_rows(current_game_num, schedule, variant));
    }

    // Context block AFTER the current block, in-game only: the upcoming game (no score).
    if !between {
        rows.push(game_block_row(
            GameRole::Next,
            &snapshot.next_game_number,
            Some(time_string(config.game_block)),
            None,
            using_uwhportal,
            schedule,
            teams,
        ));
    }
    rows
}

fn team_timeouts_value(config: &GameConfig) -> String {
    if config.num_team_timeouts_allowed == 0 {
        "0".to_string()
    } else if config.timeouts_counted_per_half {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
    } else {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
    }
}

fn stop_clock_value(schedule: Option<&Schedule>, game_number: &GameNumber) -> String {
    match schedule.and_then(|s| s.get_game_timing(game_number)) {
        Some(rule) => bool_string(rule.last_2_min_stop_time),
        None => fl!("unknown"),
    }
}

// Builds a GameBlock row. `scores` populates the team lines when present; team
// names resolve from the schedule (portal only), else None.
fn game_block_row(
    role: GameRole,
    game_number: &GameNumber,
    game_block: Option<String>,
    scores: Option<BlackWhiteBundle<u8>>,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> Row {
    let (white_name, black_name, number) =
        resolve_game(game_number, using_uwhportal, schedule, teams);
    Row::GameBlock {
        role,
        number,
        game_block,
        white: TeamLine {
            name: white_name,
            score: scores.map(|s| s.white),
        },
        black: TeamLine {
            name: black_name,
            score: scores.map(|s| s.black),
        },
    }
}

fn referee_rows(
    game_number: &GameNumber,
    schedule: Option<&Schedule>,
    variant: Variant,
) -> Vec<Row> {
    // Resolve assigned names by role; "-" for an assigned-but-unnamed or absent slot.
    let mut chief = "-".to_string();
    let mut keeper = "-".to_string();
    let mut helper: Option<String> = None;
    let mut water = ["-".to_string(), "-".to_string(), "-".to_string()];

    if let Some(game) = schedule.and_then(|s| s.games.get(game_number)) {
        if let Some(refs) = &game.referee_assignments {
            for r in refs {
                if r.user_id.is_none() {
                    continue;
                }
                let name = r.display_name.clone().unwrap_or_else(|| "-".to_string());
                match r.role.as_str() {
                    "Chief" => chief = name,
                    "TimeOrScoreKeeper" => keeper = name,
                    "TimeOrScoreKeeperHelper" => helper = Some(name),
                    "Water1" => water[0] = name,
                    "Water2" => water[1] = name,
                    "Water3" => water[2] = name,
                    _ => {}
                }
            }
        }
    }

    let mut out = vec![
        Row::Referee {
            label: fl!("gi-ref-chief"),
            name: chief,
        },
        Row::Referee {
            label: fl!("gi-ref-timekeeper"),
            name: keeper,
        },
    ];
    if matches!(variant, Variant::Compact) {
        return out; // main page: Chief + Keeper only
    }
    if let Some(h) = helper {
        out.push(Row::Referee {
            label: fl!("gi-ref-timekeeper-helper"),
            name: h,
        });
    }
    out.push(Row::Referee {
        label: fl!("gi-ref-water-1"),
        name: water[0].clone(),
    });
    out.push(Row::Referee {
        label: fl!("gi-ref-water-2"),
        name: water[1].clone(),
    });
    out.push(Row::Referee {
        label: fl!("gi-ref-water-3"),
        name: water[2].clone(),
    });
    out
}

// Returns (white_name, black_name, display_number). Names are Some only when the
// portal schedule has the game; the display number falls back to the raw number.
fn resolve_game(
    game_number: &GameNumber,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> (Option<String>, Option<String>, String) {
    if using_uwhportal {
        if let Some(game) = schedule.and_then(|s| s.games.get(game_number)) {
            let black = limit_team_name_len(&get_team_name(&game.dark, teams), TEAM_NAME_LEN_LIMIT);
            let white =
                limit_team_name_len(&get_team_name(&game.light, teams), TEAM_NAME_LEN_LIMIT);
            return (Some(white), Some(black), game.number.to_string());
        }
    }
    (None, None, game_number.to_string())
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Render the `Vec<Row>` produced by `game_info_rows` into an iced `Element`.
pub(in super::super) fn render_game_info_table(rows: Vec<Row>) -> Element<'static, Message> {
    let mut col = column![].spacing(SPACING / 4.0).width(Length::Fill);
    for r in rows {
        col = col.push(match r {
            Row::GameBlock {
                role,
                number,
                game_block,
                white,
                black,
            } => render_game_block(role, number, game_block, white, black),
            Row::SettingPair { left, right } => render_setting_pair(left, right),
            Row::Referee { label, name } => render_referee(label, name),
        });
    }
    col.into()
}

// ── Row renderers ─────────────────────────────────────────────────────────────

/// Renders a GameBlock as a 2-row × 4-column grid.
///
/// Row 1: [role label + game number] | [white team name] | [white score]
/// Row 2: [game block label+value (if Some), else blank] | [black team name] | [black score]
///
/// The black (dark) team row uses `black_container` (dark bg, light text).
/// The white (light) team row uses `white_container` (light bg, dark text).
fn render_game_block(
    role: GameRole,
    number: String,
    game_block: Option<String>,
    white: TeamLine,
    black: TeamLine,
) -> Element<'static, Message> {
    let role_label = match role {
        GameRole::Last => fl!("gi-last-game"),
        GameRole::Current => fl!("gi-current-game"),
        GameRole::Next => fl!("gi-next-game"),
    };

    // Left column, row 1: role label and game number.
    let header_cell = label_cell(format!("{role_label} {number}"));

    // Left column, row 2: game block value (only when Some).
    let block_cell: Element<'static, Message> = match game_block {
        Some(gb) => label_cell(gb),
        None => container(text("")).width(Length::Fill).into(),
    };

    let left_col: Element<'static, Message> = column![header_cell, block_cell]
        .width(Length::FillPortion(3))
        .into();

    // White team row (top, light background).
    let white_row: Element<'static, Message> = container(
        row![team_name_cell(white.name), team_score_cell(white.score),]
            .spacing(SPACING / 2.0)
            .align_y(Alignment::Center),
    )
    .padding(PADDING / 2.0)
    .width(Length::Fill)
    .style(white_container)
    .into();

    // Black team row (bottom, dark background with light text).
    let black_row: Element<'static, Message> = container(
        row![team_name_cell(black.name), team_score_cell(black.score),]
            .spacing(SPACING / 2.0)
            .align_y(Alignment::Center),
    )
    .padding(PADDING / 2.0)
    .width(Length::Fill)
    .style(black_container)
    .into();

    let right_col: Element<'static, Message> = column![white_row, black_row]
        .width(Length::FillPortion(5))
        .into();

    row![left_col, right_col]
        .spacing(SPACING / 2.0)
        .width(Length::Fill)
        .into()
}

/// Renders a SettingPair as a 4-column row: left label, left value, right label, right value.
/// When `right` is `None`, the right two cells are left blank.
fn render_setting_pair(
    left: (String, String),
    right: Option<(String, String)>,
) -> Element<'static, Message> {
    let (right_label, right_value) = match right {
        Some((l, v)) => (l, v),
        None => (String::new(), String::new()),
    };

    row![
        label_cell(left.0),
        value_cell(left.1),
        label_cell(right_label),
        value_cell(right_value),
    ]
    .spacing(SPACING / 2.0)
    .width(Length::Fill)
    .into()
}

/// Renders a Referee row as a full-width label + name.
fn render_referee(label: String, name: String) -> Element<'static, Message> {
    row![label_cell(label), value_cell(name),]
        .spacing(SPACING / 2.0)
        .width(Length::Fill)
        .into()
}

// ── Cell helpers ──────────────────────────────────────────────────────────────

/// A left-aligned label cell (grey background, standard small text).
fn label_cell(content: impl Into<String>) -> Element<'static, Message> {
    container(
        text(content.into())
            .size(SMALL_TEXT)
            .align_x(Horizontal::Left),
    )
    .padding(PADDING / 2.0)
    .width(Length::Fill)
    .style(gray_container)
    .into()
}

/// A right-aligned value cell (light-grey background, standard small text).
fn value_cell(content: impl Into<String>) -> Element<'static, Message> {
    container(
        text(content.into())
            .size(SMALL_TEXT)
            .align_x(Horizontal::Right),
    )
    .padding(PADDING / 2.0)
    .width(Length::Fill)
    .style(light_gray_container)
    .into()
}

/// A team name cell (fills remaining space, left-aligned).
fn team_name_cell(name: Option<String>) -> Element<'static, Message> {
    text(name.unwrap_or_default())
        .size(SMALL_TEXT)
        .align_x(Horizontal::Left)
        .width(Length::Fill)
        .into()
}

/// A score cell (right-aligned, shrinks to content).
fn team_score_cell(score: Option<u8>) -> Element<'static, Message> {
    text(score.map(|s| s.to_string()).unwrap_or_default())
        .size(SMALL_TEXT)
        .align_x(Horizontal::Right)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uwh_common::game_snapshot::GameSnapshot;

    fn cfg_all_on() -> GameConfig {
        GameConfig {
            single_half: false,
            overtime_allowed: true,
            sudden_death_allowed: true,
            num_team_timeouts_allowed: 1,
            ..Default::default()
        }
    }

    // Helper: collect the (label, value) pairs the settings grid emits, in order.
    fn setting_pairs(rows: &[Row]) -> Vec<(String, Option<String>)> {
        rows.iter()
            .filter_map(|r| match r {
                Row::SettingPair { left, right } => {
                    Some((left.0.clone(), right.as_ref().map(|p| p.0.clone())))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn current_block_always_present() {
        // GameSnapshot::default() is BetweenGames; the Current block is present in any state.
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(
            &snapshot,
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        assert!(rows.iter().any(|r| matches!(
            r,
            Row::GameBlock {
                role: GameRole::Current,
                game_block: Some(_),
                ..
            }
        )));
    }

    #[test]
    fn settings_order_all_features_on() {
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(
            &snapshot,
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        let pairs = setting_pairs(&rows);
        // Six rows, paired exactly as in the mockup.
        assert_eq!(
            pairs[0],
            (fl!("gi-half-length"), Some(fl!("gi-half-time-length")))
        );
        assert_eq!(
            pairs[1],
            (fl!("gi-timeouts"), Some(fl!("gi-timeout-duration")))
        );
        assert_eq!(pairs[2], (fl!("gi-overtime"), Some(fl!("gi-sudden-death"))));
        assert_eq!(
            pairs[3],
            (
                fl!("gi-pre-overtime-break"),
                Some(fl!("gi-pre-sudden-death-break"))
            )
        );
        assert_eq!(
            pairs[4],
            (
                fl!("gi-overtime-half-length"),
                Some(fl!("gi-minimum-game-break"))
            )
        );
        assert_eq!(
            pairs[5],
            (
                fl!("gi-overtime-half-time-length"),
                Some(fl!("gi-stop-clock-last-2"))
            )
        );
    }

    #[test]
    fn overtime_off_hides_overtime_rows() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            overtime_allowed: false,
            ..cfg_all_on()
        };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot,
            &config,
            false,
            None,
            None,
            None,
            Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(!labels.contains(&fl!("gi-pre-overtime-break")));
        assert!(!labels.contains(&fl!("gi-overtime-half-length")));
        assert!(!labels.contains(&fl!("gi-overtime-half-time-length")));
        assert!(labels.contains(&fl!("gi-minimum-game-break")));
        assert!(labels.contains(&fl!("gi-stop-clock-last-2")));
    }

    #[test]
    fn zero_timeouts_hides_duration() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            num_team_timeouts_allowed: 0,
            ..cfg_all_on()
        };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot,
            &config,
            false,
            None,
            None,
            None,
            Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(!labels.contains(&fl!("gi-timeout-duration")));
    }

    #[test]
    fn single_half_shows_game_length_hides_half_time() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            single_half: true,
            ..cfg_all_on()
        };
        let labels: Vec<String> = setting_pairs(&game_info_rows(
            &snapshot,
            &config,
            false,
            None,
            None,
            None,
            Variant::Full,
        ))
        .into_iter()
        .flat_map(|(l, r)| std::iter::once(l).chain(r))
        .collect();
        assert!(labels.contains(&fl!("gi-game-length")));
        assert!(!labels.contains(&fl!("gi-half-length")));
        assert!(!labels.contains(&fl!("gi-half-time-length")));
    }

    fn between_games_snapshot() -> GameSnapshot {
        // Equivalent to GameSnapshot::default() (BetweenGames), spelled out for clarity.
        GameSnapshot {
            current_period: GamePeriod::BetweenGames,
            ..GameSnapshot::default()
        }
    }

    fn in_game_snapshot() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            ..GameSnapshot::default()
        }
    }

    fn roles(rows: &[Row]) -> Vec<GameRole> {
        rows.iter()
            .filter_map(|r| match r {
                Row::GameBlock { role, .. } => Some(*role),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn between_games_shows_last_then_current_no_next() {
        let rows = game_info_rows(
            &between_games_snapshot(),
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        assert_eq!(roles(&rows).first(), Some(&GameRole::Last));
        assert!(roles(&rows).contains(&GameRole::Current));
        assert!(!roles(&rows).contains(&GameRole::Next));
    }

    #[test]
    fn in_game_shows_current_then_next_no_last() {
        let rows = game_info_rows(
            &in_game_snapshot(),
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        assert!(!roles(&rows).contains(&GameRole::Last));
        assert_eq!(roles(&rows).first(), Some(&GameRole::Current));
        assert_eq!(roles(&rows).last(), Some(&GameRole::Next));
    }

    #[test]
    fn last_block_has_no_game_block_line_and_uses_last_scores() {
        let scores = BlackWhiteBundle { black: 5, white: 3 };
        let rows = game_info_rows(
            &between_games_snapshot(),
            &cfg_all_on(),
            false,
            None,
            None,
            Some(scores),
            Variant::Full,
        );
        let last = rows
            .iter()
            .find_map(|r| match r {
                Row::GameBlock {
                    role: GameRole::Last,
                    game_block,
                    white,
                    black,
                    ..
                } => Some((game_block.clone(), white.score, black.score)),
                _ => None,
            })
            .unwrap();
        assert_eq!(last, (None, Some(3), Some(5)));
    }

    fn ref_labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .filter_map(|r| match r {
                Row::Referee { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn no_referees_without_portal() {
        let rows = game_info_rows(
            &GameSnapshot::default(),
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        assert!(ref_labels(&rows).is_empty());
    }

    #[test]
    fn compact_variant_keeps_only_chief_and_keeper() {
        // Portal on but no schedule => referee section still renders its fixed labels with "-".
        let rows = game_info_rows(
            &GameSnapshot::default(),
            &cfg_all_on(),
            true,
            None,
            None,
            None,
            Variant::Compact,
        );
        assert_eq!(
            ref_labels(&rows),
            vec![fl!("gi-ref-chief"), fl!("gi-ref-timekeeper")]
        );
    }

    #[test]
    fn full_variant_lists_standard_referees_without_helper() {
        let rows = game_info_rows(
            &GameSnapshot::default(),
            &cfg_all_on(),
            true,
            None,
            None,
            None,
            Variant::Full,
        );
        // Helper omitted when no Helper assignment is present.
        assert_eq!(
            ref_labels(&rows),
            vec![
                fl!("gi-ref-chief"),
                fl!("gi-ref-timekeeper"),
                fl!("gi-ref-water-1"),
                fl!("gi-ref-water-2"),
                fl!("gi-ref-water-3"),
            ]
        );
    }

    #[test]
    fn next_block_has_no_scores() {
        let rows = game_info_rows(
            &in_game_snapshot(),
            &cfg_all_on(),
            false,
            None,
            None,
            None,
            Variant::Full,
        );
        let next = rows
            .iter()
            .find_map(|r| match r {
                Row::GameBlock {
                    role: GameRole::Next,
                    white,
                    black,
                    game_block,
                    ..
                } => Some((game_block.is_some(), white.score, black.score)),
                _ => None,
            })
            .unwrap();
        assert_eq!(next, (true, None, None)); // Next keeps its Game Block line, no scores
    }
}
