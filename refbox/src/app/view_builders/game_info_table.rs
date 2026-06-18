use super::*;
use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{column, container, text},
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::schedule::{RefereeAssignment, Schedule, TeamList},
};

const TEAM_NAME_LEN_LIMIT: usize = 40;

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

/// One settings cell (label + value). `grayed` is true when the setting does not
/// apply to this game's config — it is shown dimmed rather than hidden, so the
/// grid keeps a fixed shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct SettingCell {
    pub label: String,
    pub value: String,
    pub grayed: bool,
}

impl SettingCell {
    fn active(label: String, value: String) -> Self {
        Self {
            label,
            value,
            grayed: false,
        }
    }

    fn maybe(label: String, value: String, grayed: bool) -> Self {
        Self {
            label,
            value,
            grayed,
        }
    }
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
        left: SettingCell,
        right: SettingCell,
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

    // The current game shows a live score only while a game is in progress; between
    // games the "current" block is the upcoming game, which has not been played yet.
    let current_scores = if between { None } else { Some(snapshot.scores) };

    // --- Current game block ---
    rows.push(game_block_row(
        GameRole::Current,
        current_game_num,
        Some(time_string(config.game_block)),
        current_scores,
        using_uwhportal,
        schedule,
        teams,
    ));

    // --- Settings grid (belongs to the current game) ---
    // Six fixed rows in fixed left/right slots so the layout never reorders.
    // Settings that don't apply to this game's config are shown greyed, not hidden.
    let single = config.single_half;
    let no_ot = !config.overtime_allowed;
    let no_sd = !config.sudden_death_allowed;
    let no_to = config.num_team_timeouts_allowed == 0;

    // Half Length / Game Length (left label depends on single-period) | Half-Time.
    let half_left = if single {
        SettingCell::active(
            fl!("gi-game-length"),
            time_string(config.half_play_duration),
        )
    } else {
        SettingCell::active(
            fl!("gi-half-length"),
            time_string(config.half_play_duration),
        )
    };
    rows.push(Row::SettingPair {
        left: half_left,
        right: SettingCell::maybe(
            fl!("gi-half-time-length"),
            time_string(config.half_time_duration),
            single,
        ),
    });
    rows.push(Row::SettingPair {
        left: SettingCell::active(fl!("gi-timeouts"), team_timeouts_value(config)),
        right: SettingCell::maybe(
            fl!("gi-timeout-duration"),
            time_string(config.team_timeout_duration),
            no_to,
        ),
    });
    rows.push(Row::SettingPair {
        left: SettingCell::active(fl!("gi-overtime"), bool_string(config.overtime_allowed)),
        right: SettingCell::active(
            fl!("gi-sudden-death"),
            bool_string(config.sudden_death_allowed),
        ),
    });
    rows.push(Row::SettingPair {
        left: SettingCell::maybe(
            fl!("gi-pre-overtime-break"),
            time_string(config.pre_overtime_break),
            no_ot,
        ),
        right: SettingCell::maybe(
            fl!("gi-pre-sudden-death-break"),
            time_string(config.pre_sudden_death_duration),
            no_sd,
        ),
    });
    rows.push(Row::SettingPair {
        left: SettingCell::maybe(
            fl!("gi-overtime-half-length"),
            time_string(config.ot_half_play_duration),
            no_ot,
        ),
        right: SettingCell::active(
            fl!("gi-minimum-game-break"),
            time_string(config.minimum_break),
        ),
    });
    rows.push(Row::SettingPair {
        left: SettingCell::maybe(
            fl!("gi-overtime-half-time-length"),
            time_string(config.ot_half_time_duration),
            no_ot,
        ),
        right: SettingCell::active(
            fl!("gi-stop-clock-last-2"),
            stop_clock_value(schedule, current_game_num),
        ),
    });

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
        rows.extend(referee_rows(current_game_num, schedule, teams));
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
        // Language-neutral placeholder when no timing rule is available (e.g. not
        // using the Portal). Table-specific; the global `unknown` key stays "Unknown".
        None => fl!("gi-unknown"),
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
    teams: Option<&TeamList>,
) -> Vec<Row> {
    let assignments = schedule
        .and_then(|s| s.games.get(game_number))
        .and_then(|g| g.referee_assignments.as_deref())
        .unwrap_or(&[]);
    referee_layout_rows(assignments, teams)
}

// Build the referee rows for a game. Two fixed layouts, all rows in the chosen
// layout always shown ("-" for empty):
//  * Team layout (a `Referees` role is present — only possible in the portal's Team
//    assignment mode): Water Referees (from `Referees`) + Deck Referees (from team
//    `TimeOrScoreKeeper`, with team `TimeOrScoreHelper` absorbed; keeper team wins
//    if they differ).
//  * Individual layout (otherwise): Chief / Time-Score Keeper / Helper / Water 1-3,
//    each filled by a person's name or a per-slot team name.
fn referee_layout_rows(assignments: &[RefereeAssignment], teams: Option<&TeamList>) -> Vec<Row> {
    // Resolve one assignment to a display string: person name, else team name, else "-".
    let resolve = |a: &RefereeAssignment| -> String {
        if a.user_id.is_some() {
            a.display_name.clone().unwrap_or_else(|| "-".to_string())
        } else if let Some(tid) = &a.team_id {
            teams
                .and_then(|t| t.get(tid).cloned())
                .unwrap_or_else(|| tid.full().to_string())
        } else {
            "-".to_string()
        }
    };

    let team_layout = assignments.iter().any(|a| a.role == "Referees");

    if team_layout {
        let mut water = "-".to_string();
        let mut deck = "-".to_string();
        let mut deck_from_keeper = false;
        for a in assignments {
            match a.role.as_str() {
                "Referees" => water = resolve(a),
                "TimeOrScoreKeeper" => {
                    deck = resolve(a);
                    deck_from_keeper = true;
                }
                "TimeOrScoreHelper" => {
                    if !deck_from_keeper {
                        deck = resolve(a);
                    }
                }
                _ => {}
            }
        }
        vec![
            Row::Referee {
                label: fl!("gi-ref-deck-referees"),
                name: deck,
            },
            Row::Referee {
                label: fl!("gi-ref-water-referees"),
                name: water,
            },
        ]
    } else {
        let mut chief = "-".to_string();
        let mut keeper = "-".to_string();
        let mut helper = "-".to_string();
        let mut water = ["-".to_string(), "-".to_string(), "-".to_string()];
        for a in assignments {
            match a.role.as_str() {
                "Chief" => chief = resolve(a),
                "TimeOrScoreKeeper" => keeper = resolve(a),
                "TimeOrScoreHelper" => helper = resolve(a),
                "Water1" => water[0] = resolve(a),
                "Water2" => water[1] = resolve(a),
                "Water3" => water[2] = resolve(a),
                _ => {}
            }
        }
        vec![
            Row::Referee {
                label: fl!("gi-ref-chief"),
                name: chief,
            },
            Row::Referee {
                label: fl!("gi-ref-timekeeper"),
                name: keeper,
            },
            Row::Referee {
                label: fl!("gi-ref-timekeeper-helper"),
                name: helper,
            },
            Row::Referee {
                label: fl!("gi-ref-water-1"),
                name: water[0].clone(),
            },
            Row::Referee {
                label: fl!("gi-ref-water-2"),
                name: water[1].clone(),
            },
            Row::Referee {
                label: fl!("gi-ref-water-3"),
                name: water[2].clone(),
            },
        ]
    }
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
        // Portal on but this game isn't in the schedule: leave the names empty.
        return (None, None, game_number.to_string());
    }
    // Not using the Portal: there are no scheduled team names, so label the rows
    // with the team colours ("White"/"Black") instead of leaving them blank.
    (
        Some(fl!("gi-team-light")),
        Some(fl!("gi-team-dark")),
        game_number.to_string(),
    )
}

// ── Renderer ──────────────────────────────────────────────────────────────────
// The table is a column of grid rows. Every row uses the same four columns —
// label (LABEL_FP) | value (VALUE_FP) | label/name (LABEL_FP) | value/score
// (VALUE_FP) — so settings values, team names, and scores all line up. Gridlines
// are the cells' own 1px borders (see theme::table_cell) rather than inter-cell
// gaps: borders don't consume layout space, so every row divides its full width by
// the same proportions and the columns align across rows no matter the cell count.

const LABEL_FP: u16 = 7; // label / team-name columns (1 and 3)
const VALUE_FP: u16 = 3; // value / score columns (2 and 4); ~25% narrower than labels
const HALF_FP: u16 = LABEL_FP + VALUE_FP; // one right-half (a team row's column span)
const CELL_PAD: f32 = PADDING / 2.0; // padding inside each cell
// Smaller than SMALL_TEXT so long labels (e.g. "Overtime Half-Time Length") fit on
// one line within their cell instead of wrapping.
const TABLE_TEXT: f32 = 15.0;
// Uniform height for every table row; the merged Last-game cell spans two of these.
// Kept tight so all parameters + referees fit on the Game Information page.
const ROW_H: f32 = 22.0;

type CellStyle = fn(&iced::Theme) -> iced::widget::container::Style;

pub(in super::super) fn render_game_info_table(rows: Vec<Row>) -> Element<'static, Message> {
    let mut table = column![].width(Length::Fill);
    for r in rows {
        match r {
            // Last game: "Last Game" + number as tall cells spanning both team
            // rows (no Game Block line), beside the white/black team rows.
            Row::GameBlock {
                role: GameRole::Last,
                number,
                white,
                black,
                ..
            } => {
                let right = column![team_row(white, false), team_row(black, true)]
                    .width(Length::FillPortion(HALF_FP));
                table = table.push(grid_row(vec![
                    tall_cell(
                        fl!("gi-prior-game"),
                        LABEL_FP,
                        table_label_cell,
                        Horizontal::Left,
                    ),
                    tall_cell(number, VALUE_FP, table_value_cell, Horizontal::Center),
                    right.into(),
                ]));
            }
            // Current / Next game: two rows — header (role + number) over Game
            // Block — each beside its team row, all on the shared 4-column grid.
            // A block with no score (an upcoming game) merges its name+score into
            // one wide name cell spanning the right half.
            Row::GameBlock {
                role,
                number,
                game_block,
                white,
                black,
            } => {
                let role_label = match role {
                    GameRole::Current => fl!("gi-current-game"),
                    _ => fl!("gi-next-game"),
                };
                let block = game_block.unwrap_or_default();
                let has_score = white.score.is_some() || black.score.is_some();

                let mut white_row = vec![
                    label_cell(role_label, LABEL_FP),
                    value_cell(number, VALUE_FP),
                ];
                let mut black_row = vec![
                    label_cell(fl!("gi-game-block"), LABEL_FP),
                    value_cell(block, VALUE_FP),
                ];
                if has_score {
                    white_row.push(name_cell(white.name, false, LABEL_FP));
                    white_row.push(score_cell(white.score, false, VALUE_FP));
                    black_row.push(name_cell(black.name, true, LABEL_FP));
                    black_row.push(score_cell(black.score, true, VALUE_FP));
                } else {
                    white_row.push(name_cell(white.name, false, HALF_FP));
                    black_row.push(name_cell(black.name, true, HALF_FP));
                }
                table = table.push(grid_row(white_row));
                table = table.push(grid_row(black_row));
            }
            Row::SettingPair { left, right } => {
                table = table.push(grid_row(vec![
                    setting_label(left.label, left.grayed),
                    setting_value(left.value, left.grayed),
                    setting_label(right.label, right.grayed),
                    setting_value(right.value, right.grayed),
                ]));
            }
            Row::Referee { label, name } => {
                // Label in column 1; the name (left-aligned like a name, not a
                // value) spans columns 2–4. Uses a label-style cell so it stays
                // left while value/score cells centre.
                table = table.push(grid_row(vec![
                    label_cell(label, LABEL_FP),
                    label_cell(name, VALUE_FP + HALF_FP),
                ]));
            }
        }
    }

    container(table)
        .style(table_grid_container)
        // 1px of dark backing around the cells; with each edge cell's own 1px
        // border this makes the outer frame the same 2px weight as inner gridlines.
        .padding(1.0)
        .width(Length::Fill)
        .into()
}

// ── Cell helpers ──────────────────────────────────────────────────────────────

/// A horizontal grid row; cells abut and their 1px borders form the gridlines.
fn grid_row(cells: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    iced::widget::Row::with_children(cells)
        .width(Length::Fill)
        .into()
}

/// One team row: name (column 3) + score (column 4), white or black styled.
fn team_row(line: TeamLine, dark: bool) -> Element<'static, Message> {
    grid_row(vec![
        name_cell(line.name, dark, LABEL_FP),
        score_cell(line.score, dark, VALUE_FP),
    ])
}

/// A label cell (medium-grey fill), left-aligned.
fn label_cell(content: impl Into<String>, fp: u16) -> Element<'static, Message> {
    cell(
        content.into(),
        fp,
        table_label_cell,
        Horizontal::Left,
        false,
    )
}

/// A value cell (lighter-grey fill), centre-aligned.
fn value_cell(content: impl Into<String>, fp: u16) -> Element<'static, Message> {
    cell(
        content.into(),
        fp,
        table_value_cell,
        Horizontal::Center,
        false,
    )
}

/// A team-name cell (white or black fill), left-aligned.
fn name_cell(name: Option<String>, dark: bool, fp: u16) -> Element<'static, Message> {
    cell(
        name.unwrap_or_default(),
        fp,
        team_style(dark),
        Horizontal::Left,
        false,
    )
}

/// A score cell (white or black fill), centre-aligned.
fn score_cell(score: Option<u8>, dark: bool, fp: u16) -> Element<'static, Message> {
    cell(
        score.map(|s| s.to_string()).unwrap_or_default(),
        fp,
        team_style(dark),
        Horizontal::Center,
        false,
    )
}

/// A settings label cell — greyed when the setting is inactive for this game.
fn setting_label(content: String, grayed: bool) -> Element<'static, Message> {
    let style = if grayed {
        table_label_cell_grayed
    } else {
        table_label_cell
    };
    cell(content, LABEL_FP, style, Horizontal::Left, false)
}

/// A settings value cell — greyed when the setting is inactive for this game.
fn setting_value(content: String, grayed: bool) -> Element<'static, Message> {
    let style = if grayed {
        table_value_cell_grayed
    } else {
        table_value_cell
    };
    cell(content, VALUE_FP, style, Horizontal::Center, false)
}

/// A cell that spans two rows (vertically centred) — used for the merged
/// Last-game label/number beside its two team rows. `align_x` lets the label
/// stay left while the number column centres like other value cells.
fn tall_cell(
    content: impl Into<String>,
    fp: u16,
    style: CellStyle,
    align_x: Horizontal,
) -> Element<'static, Message> {
    cell(content.into(), fp, style, align_x, true)
}

fn team_style(dark: bool) -> CellStyle {
    if dark {
        table_black_cell
    } else {
        table_white_cell
    }
}

/// Builds one filled, square table cell of uniform height `ROW_H`. `span2` makes
/// it two rows tall for the merged Last-game cells (the two stacked team rows abut,
/// so the span is exactly twice the row height).
fn cell(
    content: String,
    fp: u16,
    style: CellStyle,
    align_x: Horizontal,
    span2: bool,
) -> Element<'static, Message> {
    let height = if span2 {
        Length::Fixed(2.0 * ROW_H)
    } else {
        Length::Fixed(ROW_H)
    };
    // Align the text via the CONTAINER, not via a Fill-width `text` with its own
    // `align_x`. Anchoring alignment on the text triggers an iced 0.13
    // paragraph-cache/stale-anchor bug: two identically-shaped short texts (e.g. a
    // white "0" and a black "0") collide and one renders blank. Letting the parent
    // container position the text avoids it. See the same fix in main_view.rs.
    container(text(content).size(TABLE_TEXT))
        // Horizontal inset only; the fixed row height + vertical centring provide the
        // (now tighter) vertical spacing, so rows stay short.
        .padding([0.0, CELL_PAD])
        .width(Length::FillPortion(fp))
        .height(height)
        .align_x(align_x)
        .align_y(Vertical::Center)
        .style(style)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use uwh_common::game_snapshot::GameSnapshot;
    use uwh_common::uwhportal::schedule::TeamId;

    fn cfg_all_on() -> GameConfig {
        GameConfig {
            single_half: false,
            overtime_allowed: true,
            sudden_death_allowed: true,
            num_team_timeouts_allowed: 1,
            ..Default::default()
        }
    }

    // Helper: collect the (left-label, right-label) pairs the settings grid emits, in order.
    fn setting_pairs(rows: &[Row]) -> Vec<(String, String)> {
        rows.iter()
            .filter_map(|r| match r {
                Row::SettingPair { left, right } => Some((left.label.clone(), right.label.clone())),
                _ => None,
            })
            .collect()
    }

    // Helper: find a settings cell's `grayed` flag by label (left or right slot).
    fn cell_grayed(rows: &[Row], label: &str) -> Option<bool> {
        rows.iter().find_map(|r| match r {
            Row::SettingPair { left, right } => {
                if left.label == label {
                    Some(left.grayed)
                } else if right.label == label {
                    Some(right.grayed)
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    #[test]
    fn current_block_always_present() {
        // GameSnapshot::default() is BetweenGames; the Current block is present in any state.
        let snapshot = GameSnapshot::default();
        let rows = game_info_rows(&snapshot, &cfg_all_on(), false, None, None, None);
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
        let rows = game_info_rows(&snapshot, &cfg_all_on(), false, None, None, None);
        let pairs = setting_pairs(&rows);
        // Six fixed rows, paired exactly as in the mockup.
        assert_eq!(pairs.len(), 6);
        assert_eq!(
            pairs[0],
            (fl!("gi-half-length"), fl!("gi-half-time-length"))
        );
        assert_eq!(pairs[1], (fl!("gi-timeouts"), fl!("gi-timeout-duration")));
        assert_eq!(pairs[2], (fl!("gi-overtime"), fl!("gi-sudden-death")));
        assert_eq!(
            pairs[3],
            (
                fl!("gi-pre-overtime-break"),
                fl!("gi-pre-sudden-death-break")
            )
        );
        assert_eq!(
            pairs[4],
            (fl!("gi-overtime-half-length"), fl!("gi-minimum-game-break"))
        );
        assert_eq!(
            pairs[5],
            (
                fl!("gi-overtime-half-time-length"),
                fl!("gi-stop-clock-last-2")
            )
        );
    }

    #[test]
    fn overtime_off_grays_overtime_settings() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            overtime_allowed: false,
            ..cfg_all_on()
        };
        let rows = game_info_rows(&snapshot, &config, false, None, None, None);
        // Overtime-detail settings stay in their slots but greyed.
        assert_eq!(
            cell_grayed(&rows, &fl!("gi-pre-overtime-break")),
            Some(true)
        );
        assert_eq!(
            cell_grayed(&rows, &fl!("gi-overtime-half-length")),
            Some(true)
        );
        assert_eq!(
            cell_grayed(&rows, &fl!("gi-overtime-half-time-length")),
            Some(true)
        );
        // Always-applicable settings stay active.
        assert_eq!(
            cell_grayed(&rows, &fl!("gi-minimum-game-break")),
            Some(false)
        );
        assert_eq!(
            cell_grayed(&rows, &fl!("gi-stop-clock-last-2")),
            Some(false)
        );
        // Fixed pairing preserved: Overtime stays beside Sudden Death.
        assert!(setting_pairs(&rows).contains(&(fl!("gi-overtime"), fl!("gi-sudden-death"))));
    }

    #[test]
    fn zero_timeouts_grays_duration() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            num_team_timeouts_allowed: 0,
            ..cfg_all_on()
        };
        let rows = game_info_rows(&snapshot, &config, false, None, None, None);
        assert_eq!(cell_grayed(&rows, &fl!("gi-timeout-duration")), Some(true));
        assert_eq!(cell_grayed(&rows, &fl!("gi-timeouts")), Some(false));
    }

    #[test]
    fn single_half_shows_game_length_grays_half_time() {
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            single_half: true,
            ..cfg_all_on()
        };
        let rows = game_info_rows(&snapshot, &config, false, None, None, None);
        let pairs = setting_pairs(&rows);
        // First row's left label becomes "Game Length" (active); "Half Length" is unused.
        assert_eq!(pairs[0].0, fl!("gi-game-length"));
        assert_eq!(cell_grayed(&rows, &fl!("gi-half-length")), None);
        // Half-Time keeps its slot but greyed.
        assert_eq!(cell_grayed(&rows, &fl!("gi-half-time-length")), Some(true));
    }

    #[test]
    fn settings_keep_six_fixed_rows_when_mostly_off() {
        // The user's reported bug: with OT/SD off and 0 timeouts, rows must NOT
        // reflow into wrong pairs — every slot stays fixed (just greyed).
        let snapshot = GameSnapshot::default();
        let config = GameConfig {
            single_half: false,
            overtime_allowed: false,
            sudden_death_allowed: false,
            num_team_timeouts_allowed: 0,
            ..Default::default()
        };
        let rows = game_info_rows(&snapshot, &config, false, None, None, None);
        let pairs = setting_pairs(&rows);
        assert_eq!(pairs.len(), 6);
        assert!(pairs.contains(&(fl!("gi-overtime"), fl!("gi-sudden-death"))));
        assert!(pairs.contains(&(fl!("gi-timeouts"), fl!("gi-timeout-duration"))));
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
        );
        assert_eq!(roles(&rows).first(), Some(&GameRole::Last));
        assert!(roles(&rows).contains(&GameRole::Current));
        assert!(!roles(&rows).contains(&GameRole::Next));
    }

    #[test]
    fn in_game_shows_current_then_next_no_last() {
        let rows = game_info_rows(&in_game_snapshot(), &cfg_all_on(), false, None, None, None);
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

    fn person(role: &str, name: &str) -> RefereeAssignment {
        RefereeAssignment {
            role: role.to_string(),
            user_id: Some(format!("uid-{name}")),
            team_id: None,
            display_name: Some(name.to_string()),
        }
    }
    fn team(role: &str, full_id: &str) -> RefereeAssignment {
        RefereeAssignment {
            role: role.to_string(),
            user_id: None,
            team_id: Some(TeamId::from_full(full_id).unwrap()),
            display_name: None,
        }
    }
    fn teamlist() -> TeamList {
        BTreeMap::from([(
            TeamId::from_full("teams/10753-A").unwrap(),
            "Sharks".to_string(),
        )])
    }
    fn ref_pairs(rows: &[Row]) -> Vec<(String, String)> {
        rows.iter()
            .filter_map(|r| match r {
                Row::Referee { label, name } => Some((label.clone(), name.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn team_layout_shows_water_and_deck_referees() {
        let tl = teamlist();
        let rows = referee_layout_rows(
            &[
                team("Referees", "teams/10753-A"),
                team("TimeOrScoreKeeper", "teams/10753-A"),
            ],
            Some(&tl),
        );
        assert_eq!(
            ref_pairs(&rows),
            vec![
                (fl!("gi-ref-deck-referees"), "Sharks".to_string()),
                (fl!("gi-ref-water-referees"), "Sharks".to_string()),
            ]
        );
    }

    #[test]
    fn individual_layout_resolves_person_and_corrected_helper_role() {
        let rows = referee_layout_rows(
            &[person("Chief", "Alice"), person("TimeOrScoreHelper", "Bob")],
            None,
        );
        let pairs = ref_pairs(&rows);
        assert_eq!(pairs.len(), 6); // all individual rows always shown
        assert!(pairs.contains(&(fl!("gi-ref-chief"), "Alice".to_string())));
        // Corrected role string ("TimeOrScoreHelper") now resolves into the helper row.
        assert!(pairs.contains(&(fl!("gi-ref-timekeeper-helper"), "Bob".to_string())));
    }

    #[test]
    fn team_in_numbered_water_slot_is_per_slot() {
        let tl = teamlist();
        let rows = referee_layout_rows(&[team("Water2", "teams/10753-A")], Some(&tl));
        let pairs = ref_pairs(&rows);
        assert!(pairs.contains(&(fl!("gi-ref-water-2"), "Sharks".to_string())));
        assert_eq!(pairs.len(), 6); // individual layout (no Referees role)
    }

    #[test]
    fn deck_referees_absorbs_team_helper_keeper_wins() {
        let mut tl = teamlist();
        tl.insert(
            TeamId::from_full("teams/999-B").unwrap(),
            "Rays".to_string(),
        );
        let rows = referee_layout_rows(
            &[
                team("Referees", "teams/10753-A"),
                team("TimeOrScoreKeeper", "teams/10753-A"),
                team("TimeOrScoreHelper", "teams/999-B"),
            ],
            Some(&tl),
        );
        // Keeper team wins the single Deck Referees row; helper absorbed.
        assert_eq!(
            ref_pairs(&rows),
            vec![
                (fl!("gi-ref-deck-referees"), "Sharks".to_string()),
                (fl!("gi-ref-water-referees"), "Sharks".to_string()),
            ]
        );
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
        );
        assert!(ref_labels(&rows).is_empty());
    }

    #[test]
    fn referee_rows_always_include_blank_helper_and_all_water() {
        // Portal on but no schedule => referee section renders its fixed labels with "-".
        // Both tables list the same referees, including a blank T/S Helper row.
        let rows = game_info_rows(
            &GameSnapshot::default(),
            &cfg_all_on(),
            true,
            None,
            None,
            None,
        );
        assert_eq!(
            ref_labels(&rows),
            vec![
                fl!("gi-ref-chief"),
                fl!("gi-ref-timekeeper"),
                fl!("gi-ref-timekeeper-helper"),
                fl!("gi-ref-water-1"),
                fl!("gi-ref-water-2"),
                fl!("gi-ref-water-3"),
            ]
        );
    }

    #[test]
    fn next_block_has_no_scores() {
        let rows = game_info_rows(&in_game_snapshot(), &cfg_all_on(), false, None, None, None);
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

    #[test]
    fn between_games_current_block_has_no_score() {
        // Between games the "Current" block is the upcoming (not-yet-played) game.
        let rows = game_info_rows(
            &between_games_snapshot(),
            &cfg_all_on(),
            false,
            None,
            None,
            None,
        );
        let current = rows
            .iter()
            .find_map(|r| match r {
                Row::GameBlock {
                    role: GameRole::Current,
                    white,
                    black,
                    ..
                } => Some((white.score, black.score)),
                _ => None,
            })
            .unwrap();
        assert_eq!(current, (None, None));
    }

    #[test]
    fn in_game_current_block_carries_live_score() {
        let snapshot = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            scores: BlackWhiteBundle { black: 2, white: 3 },
            ..GameSnapshot::default()
        };
        let rows = game_info_rows(&snapshot, &cfg_all_on(), false, None, None, None);
        let current = rows
            .iter()
            .find_map(|r| match r {
                Row::GameBlock {
                    role: GameRole::Current,
                    white,
                    black,
                    ..
                } => Some((white.score, black.score)),
                _ => None,
            })
            .unwrap();
        assert_eq!(current, (Some(3), Some(2)));
    }
}
