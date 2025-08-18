use super::*;
use crate::app::dynamic_font_sizing::{DynamicFontSizing, GameInfoCell};
use iced::{
    Alignment, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, container, row, text},
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot, PenaltyTime},
    uwhportal::schedule::{GameList, TeamList},
};
use std::collections::HashMap;

/// Test data for demonstrating font sizing in the GUI
pub struct FontSizingTestData {
    pub team_names: HashMap<String, String>,
    pub referee_names: HashMap<String, String>,
}

impl FontSizingTestData {
    /// Create test data with your specification values
    pub fn specification_data() -> Self {
        let mut team_names = HashMap::new();
        team_names.insert("WHITE".to_string(), "Australia".to_string());
        team_names.insert("BLACK".to_string(), "New Zealand".to_string());
        team_names.insert("NEXT_WHITE".to_string(), "Nederlands".to_string());
        team_names.insert("NEXT_BLACK".to_string(), "South Africa".to_string());

        let mut referee_names = HashMap::new();
        referee_names.insert("Chief Ref".to_string(), "Russell Owen Camilo La Torre".to_string());
        referee_names.insert("Timer".to_string(), "Norfatin Aainaa Binti Hashim".to_string());
        referee_names.insert("Water Ref 1".to_string(), "Tuan San Jonathan Chan".to_string());
        referee_names.insert("Water Ref 2".to_string(), "Muhammad Danish Haikal Mohd Fadel".to_string());
        referee_names.insert("Water Ref 3".to_string(), "A very long person name".to_string());

        Self {
            team_names,
            referee_names,
        }
    }

    /// Create test data with short names (should not trigger font reduction)
    pub fn short_names_data() -> Self {
        let mut team_names = HashMap::new();
        team_names.insert("WHITE".to_string(), "USA".to_string());
        team_names.insert("BLACK".to_string(), "UK".to_string());
        team_names.insert("NEXT_WHITE".to_string(), "France".to_string());
        team_names.insert("NEXT_BLACK".to_string(), "Spain".to_string());

        let mut referee_names = HashMap::new();
        referee_names.insert("Chief Ref".to_string(), "John Smith".to_string());
        referee_names.insert("Timer".to_string(), "Jane Doe".to_string());
        referee_names.insert("Water Ref 1".to_string(), "Bob Wilson".to_string());
        referee_names.insert("Water Ref 2".to_string(), "Sue Chen".to_string());
        referee_names.insert("Water Ref 3".to_string(), "Tom Brown".to_string());

        Self {
            team_names,
            referee_names,
        }
    }
}

/// Helper function to get the appropriate font size for a table row value
fn get_font_size_for_table_row(
    label: &str,
    dynamic_font_sizing: &DynamicFontSizing,
) -> f32 {
    // Map table row labels to GameInfoCell enum
    let cell = match label {
        "WHITE" => Some(GameInfoCell::NextGame),  // WHITE team uses NextGame cell
        "BLACK" => Some(GameInfoCell::LastGame),  // BLACK team uses LastGame cell
        "Chief Ref" => Some(GameInfoCell::ChiefRef),
        "Timer" => Some(GameInfoCell::Timer),
        "Water Ref 1" => Some(GameInfoCell::WaterRef1),
        "Water Ref 2" => Some(GameInfoCell::WaterRef2),
        "Water Ref 3" => Some(GameInfoCell::WaterRef3),
        _ => None,
    };

    // Return dynamic font size for target cells, default size for others
    if let Some(cell) = cell {
        dynamic_font_sizing.get_font_size(cell)
    } else {
        SMALL_TEXT
    }
}

// Constants for fixed-width label cells
const GAME_LABEL_WIDTH: f32 = 100.0;
const REF_LABEL_WIDTH: f32 = 120.0;
// Fixed cell height to keep rows from shrinking when font sizes are reduced
const GAME_INFO_CELL_HEIGHT: f32 = 26.0;




#[derive(Debug, Clone)]
struct TableRow {
    left_label: String,
    left_value: String,
    center_label: Option<String>,
    center_value: Option<String>,
}

pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    track_fouls_and_warnings: bool,
    dynamic_font_sizing: &mut DynamicFontSizing,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        font_demo,
        demo_data_type,
    } = data;

    let time_button = make_game_time_button(snapshot, true, false, mode, clock_running);

    let mut center_col = column![time_button].spacing(SPACING).width(Length::Fill);

    let make_warn_button = || {
        make_button(fl!("add-warning"))
            .style(blue_button)
            .width(Length::Fill)
            .on_press(Message::KeypadPage(KeypadPage::WarningAdd {
                origin: None,
                color: GameColor::Black,
                infraction: Infraction::Unknown,
                team_warning: false,
                ret_to_overview: false,
            }))
    };

    let make_foul_button = || {
        make_button(fl!("add-foul"))
            .style(orange_button)
            .width(Length::Fill)
            .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                origin: None,
                color: None,
                infraction: Infraction::Unknown,
                ret_to_overview: false,
            }))
    };

    if snapshot.timeout.is_some() {
        if track_fouls_and_warnings {
            center_col =
                center_col.push(row![make_foul_button(), make_warn_button()].spacing(SPACING));
        } else {
            center_col = center_col.push(
                make_button(fl!("end-timeout"))
                    .style(yellow_button)
                    .on_press(Message::EndTimeout),
            );
        }
    } else {
        match snapshot.current_period {
            GamePeriod::BetweenGames
            | GamePeriod::HalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreSuddenDeath => {
                let mut start_warning_row = row![
                    make_button(fl!("start-now"))
                        .style(green_button)
                        .width(Length::Fill)
                        .on_press(Message::StartPlayNow)
                ]
                .spacing(SPACING);

                if track_fouls_and_warnings {
                    start_warning_row = start_warning_row.push(make_warn_button())
                }

                center_col = center_col.push(start_warning_row)
            }
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath => {
                if track_fouls_and_warnings {
                    center_col = center_col
                        .push(row![make_foul_button(), make_warn_button()].spacing(SPACING))
                }
            }
        };
    }

    let max_num_warns = snapshot
        .warnings
        .iter()
        .map(|(_, w)| w.len())
        .max()
        .unwrap();

    center_col = center_col.push(if max_num_warns < 4 {
        button(build_config_table(
            snapshot,
            game_config,
            using_uwhportal,
            games,
            teams,
            track_fouls_and_warnings,
            dynamic_font_sizing,
            font_demo,
            &demo_data_type,
        ))
        .padding(PADDING)
        .style(light_gray_button)
        .height(Length::FillPortion(2))
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails)
    } else {
        button(
            text(config_string_game_num(snapshot, using_uwhportal, games).0)
                .size(SMALL_TEXT)
                .align_y(Vertical::Center)
                .align_x(Horizontal::Left),
        )
        .padding(PADDING)
        .style(light_gray_button)
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails)
    });

    if track_fouls_and_warnings {
        center_col = center_col.push(
            button(
                column![
                    text(fl!("warnings"))
                        .align_y(Vertical::Top)
                        .align_x(Horizontal::Center)
                        .width(Length::Fill),
                    row(snapshot.warnings.iter().map(|(color, warns)| column(
                        warns
                            .iter()
                            .rev()
                            .take(10)
                            .map(|warning| make_warning_container(warning, Some(color)).into())
                    )
                    .spacing(1)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()))
                    .spacing(SPACING),
                ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::NoAction)
            .style(light_gray_button)
            .on_press(Message::ShowWarnings),
        )
    }

    let make_penalty_button = |snapshot: &GameSnapshot, color: GameColor| {
        let penalties = &snapshot.penalties[color];

        let time = penalties
            .iter()
            .filter_map(|penalty| match penalty.time {
                PenaltyTime::Seconds(s) if s != 0 => Some(s),
                PenaltyTime::Seconds(_) => None,
                PenaltyTime::TotalDismissal => None,
            })
            .min();

        let make_penalties_red = if snapshot.timeout.is_none() {
            if let Some(t) = time {
                t <= 10 && (t % 2 == 0) && (t != 0)
            } else {
                false
            }
        } else {
            false
        };

        let button_style = if make_penalties_red {
            red_button
        } else {
            match color {
                GameColor::Black => black_button,
                GameColor::White => white_button,
            }
        };

        button(
            column![
                text(fl!("penalties"))
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
                text(penalty_string(penalties))
                    .align_y(Vertical::Top)
                    .align_x(Horizontal::Left)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::PenaltyOverview)
        .style(button_style)
    };

    let mut black_score_btn = button(
        column![
            text(fl!("dark-team-name-caps")),
            text(snapshot.scores.black.to_string()).size(LARGE_TEXT),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(black_button);

    let mut black_new_score_btn =
        make_multi_label_button((fl!("dark-score-line-1"), fl!("dark-score-line-2")))
            .style(black_button);

    let mut white_score_btn = button(
        column![
            text(fl!("light-team-name-caps")),
            text(snapshot.scores.white.to_string()).size(LARGE_TEXT),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(white_button);

    let mut white_new_score_btn =
        make_multi_label_button((fl!("light-score-line-1"), fl!("light-score-line-2")))
            .style(white_button);

    if snapshot.current_period != GamePeriod::BetweenGames {
        black_score_btn = black_score_btn.on_press(Message::EditScores);
        black_new_score_btn = black_new_score_btn.on_press(Message::AddNewScore(GameColor::Black));
        white_score_btn = white_score_btn.on_press(Message::EditScores);
        white_new_score_btn = white_new_score_btn.on_press(Message::AddNewScore(GameColor::White));
    }

    let black_col = column![
        black_score_btn,
        black_new_score_btn,
        make_penalty_button(snapshot, GameColor::Black),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill);

    let white_col = column![
        white_score_btn,
        white_new_score_btn,
        make_penalty_button(snapshot, GameColor::White),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill);

    row![
        row![
            black_col,
            Space::with_width(Length::Fixed(3.0 * SPACING / 4.0)),
        ]
        .width(Length::Fill)
        .spacing(0),
        row![
            Space::with_width(Length::Fixed(SPACING / 4.0)),
            center_col,
            Space::with_width(Length::Fixed(SPACING / 4.0)),
        ]
        .width(Length::FillPortion(2))
        .spacing(0),
        row![
            Space::with_width(Length::Fixed(3.0 * SPACING / 4.0)),
            white_col,
        ]
        .width(Length::Fill)
        .spacing(0),
    ]
    .spacing(0)
    .height(Length::Fill)
    .into()
}

fn build_config_table<'a>(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    teams: Option<&TeamList>,
    fouls_and_warnings: bool,
    dynamic_font_sizing: &mut DynamicFontSizing,
    font_demo: bool,
    demo_data_type: &str,
) -> Element<'a, Message> {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut table_rows = Vec::new();

    // Always show Game Number as Row 1
    let _game_display = if using_uwhportal {
        if let Some(games) = games {
            match games.get(&snapshot.game_number) {
                Some(game) => game.number.to_string(),
                None if snapshot.game_number == "0" => fl!("none").to_string(),
                None => fl!(
                    "game-number-error",
                    game_number = snapshot.game_number.clone()
                ),
            }
        } else {
            fl!(
                "game-number-error",
                game_number = snapshot.game_number.clone()
            )
        }
    } else {
        if snapshot.game_number == "0" {
            fl!("none").to_string()
        } else {
            snapshot.game_number.to_string()
        }
    };

    // Game number information for reference
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        &snapshot.next_game_number
    } else {
        &snapshot.game_number
    };

    // Get team names for current/next game
    let (current_black_team, current_white_team) = if using_uwhportal {
        if let Some(games) = games {
            if let Some(game) = games.get(game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                (
                    limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT),
                )
            } else {
                ("None".to_string(), "None".to_string())
            }
        } else {
            ("None".to_string(), "None".to_string())
        }
    } else {
        ("None".to_string(), "None".to_string())
    };

    // Get team names for previous game (if available)
    let (_prev_black_team, _prev_white_team) = if using_uwhportal {
        if let Some(games) = games {
            // Try to get previous game number (current game number - 1)
            let prev_game_num = if let Ok(current_num) = snapshot.game_number.parse::<i32>() {
                if current_num > 1 {
                    (current_num - 1).to_string()
                } else {
                    "0".to_string()
                }
            } else {
                "0".to_string()
            };

            if let Some(prev_game) = games.get(&prev_game_num) {
                let black = get_team_name(&prev_game.dark, teams);
                let white = get_team_name(&prev_game.light, teams);
                (
                    limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT),
                )
            } else {
                ("None".to_string(), "None".to_string())
            }
        } else {
            ("None".to_string(), "None".to_string())
        }
    } else {
        ("None".to_string(), "None".to_string())
    };

    // Add "Last Game" rows - first row with White team
    table_rows.push(TableRow {
        left_label: "Last Game".to_string(),
        left_value: "White".to_string(),
        center_label: Some(_prev_white_team.clone()),
        center_value: None,
    });

    // Add second row for Last Game with Black team (empty label)
    table_rows.push(TableRow {
        left_label: "".to_string(),
        left_value: "Black".to_string(),
        center_label: Some(_prev_black_team.clone()),
        center_value: None,
    });

    // Add "Next Game" rows - first row with White team
    table_rows.push(TableRow {
        left_label: "Next Game".to_string(),
        left_value: "White".to_string(),
        center_label: Some(current_white_team.clone()),
        center_value: None,
    });

    // Add second row for Next Game with Black team (empty label)
    table_rows.push(TableRow {
        left_label: "".to_string(),
        left_value: "Black".to_string(),
        center_label: Some(current_black_team.clone()),
        center_value: None,
    });

    // Compact layout - everything in fewer rows to match screenshot
    // Row: Half Duration | 15:00 | Half Time Duration | 3:00
    table_rows.push(TableRow {
        left_label: "Half Duration".to_string(),
        left_value: time_string(config.half_play_duration),
        center_label: Some("Half Time Duration".to_string()),
        center_value: Some(time_string(config.half_time_duration)),
    });

    // Row: Overtime | YES | Sudden Death | YES (swapped positions)
    table_rows.push(TableRow {
        left_label: "Overtime".to_string(),
        left_value: bool_string(config.overtime_allowed),
        center_label: Some("Sudden Death".to_string()),
        center_value: Some(bool_string(config.sudden_death_allowed)),
    });

    // Row 3: Timeouts Per Half | 1 (single column)
    // Row: Timeouts | 1 / Game | Last 2 Min Stop Time | YES
    let timeout_value = if config.timeouts_counted_per_half {
        format!("{} / Half", config.num_team_timeouts_allowed)
    } else if config.num_team_timeouts_allowed == 0 {
        "None".to_string()
    } else {
        format!("{} / Game", config.num_team_timeouts_allowed)
    };

    table_rows.push(TableRow {
        left_label: "Timeouts".to_string(),
        left_value: timeout_value,
        center_label: Some("Last 2 Min Ref T/Out".to_string()),
        center_value: Some("YES".to_string()), // Default value, will be configurable later
    });

    // Officials information - compact layout to match screenshot
    if !fouls_and_warnings {
        let unknown = fl!("unknown");

        // Check if we're in demo mode and get test data
        let demo_data = if font_demo {
            match demo_data_type {
                "short" => Some(FontSizingTestData::short_names_data()),
                _ => Some(FontSizingTestData::specification_data()),
            }
        } else {
            None
        };

        // Row 5: Chief Ref | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Chief Ref".to_string(),
            left_value: demo_data.as_ref()
                .and_then(|d| d.referee_names.get("Chief Ref"))
                .cloned()
                .unwrap_or_else(|| unknown.clone()),
            center_label: None,
            center_value: None,
        });

        // Row 6: Timer | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Timer".to_string(),
            left_value: demo_data.as_ref()
                .and_then(|d| d.referee_names.get("Timer"))
                .cloned()
                .unwrap_or_else(|| unknown.clone()),
            center_label: None,
            center_value: None,
        });

        // Row 7: Water Ref 1 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 1".to_string(),
            left_value: demo_data.as_ref()
                .and_then(|d| d.referee_names.get("Water Ref 1"))
                .cloned()
                .unwrap_or_else(|| unknown.clone()),
            center_label: None,
            center_value: None,
        });

        // Row 8: Water Ref 2 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 2".to_string(),
            left_value: demo_data.as_ref()
                .and_then(|d| d.referee_names.get("Water Ref 2"))
                .cloned()
                .unwrap_or_else(|| unknown.clone()),
            center_label: None,
            center_value: None,
        });

        // Row 9: Water Ref 3 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 3".to_string(),
            left_value: demo_data.as_ref()
                .and_then(|d| d.referee_names.get("Water Ref 3"))
                .cloned()
                .unwrap_or_else(|| unknown.clone()),
            center_label: None,
            center_value: None,
        });
    }

    // Update dynamic font sizing with the current cell values
    // Update font sizes for all target cells with their current text content
    let mut current_section: Option<&str> = None;
    let mut last_game_max: Option<String> = None;
    let mut next_game_max: Option<String> = None;

    for table_row in &table_rows {
        // Track the current game section so both White/Black rows share the same font size
        if table_row.left_label == "Last Game" { current_section = Some("Last Game"); }
        if table_row.left_label == "Next Game" { current_section = Some("Next Game"); }

        // Accumulate the longest team name within each section (Last/Next Game)
        if (table_row.left_label == "Last Game" || table_row.left_label == "Next Game" || table_row.left_label.is_empty())
            && (table_row.left_value == "White" || table_row.left_value == "Black")
        {
            if let Some(center_text) = &table_row.center_label {
                match current_section {
                    Some("Next Game") => {
                        if next_game_max.as_ref().map(|s| s.len()).unwrap_or(0) < center_text.len() {
                            next_game_max = Some(center_text.clone());
                        }
                    }
                    _ => {
                        if last_game_max.as_ref().map(|s| s.len()).unwrap_or(0) < center_text.len() {
                            last_game_max = Some(center_text.clone());
                        }
                    }
                }
            }
        }

        // Handle other target cells
        match table_row.left_label.as_str() {
            "Chief Ref" => {
                dynamic_font_sizing.update_cell_font_size(GameInfoCell::ChiefRef, &table_row.left_value);
            }
            "Timer" => {
                dynamic_font_sizing.update_cell_font_size(GameInfoCell::Timer, &table_row.left_value);
            }
            "Water Ref 1" => {
                dynamic_font_sizing.update_cell_font_size(GameInfoCell::WaterRef1, &table_row.left_value);
            }
            "Water Ref 2" => {
                dynamic_font_sizing.update_cell_font_size(GameInfoCell::WaterRef2, &table_row.left_value);
            }
            "Water Ref 3" => {
                dynamic_font_sizing.update_cell_font_size(GameInfoCell::WaterRef3, &table_row.left_value);
            }
            _ => {} // Non-target cells don't need font size updates
        }
    }

    // Now update font sizing once per section with the longest name so both rows match
    if let Some(text) = last_game_max.as_ref() {
        dynamic_font_sizing.update_cell_font_size(GameInfoCell::LastGame, text);
    }
    if let Some(text) = next_game_max.as_ref() {
        dynamic_font_sizing.update_cell_font_size(GameInfoCell::NextGame, text);
    }

    // Build the table layout with compact spacing to fit all content
    let mut details_column = column![]
        .spacing(SPACING / 4.0) // Much smaller spacing to fit more rows
        .width(Length::Fill);

    // Track which game section we're in so both rows (White/Black) share the same font sizing
    let mut current_game_header: Option<String> = None;

    for table_row in table_rows {
        // Check if we have center content to create multi-column row
        if let Some(center_label) = table_row.center_label {
            // Check if this is a 4-column row (Label|Value|Label|Value) or 3-column row (Label|Value|TeamName)
            let is_four_column = table_row.center_value.is_some();

            if is_four_column {
                let center_value = table_row.center_value.unwrap();
                // Create a 4-column row: Label | Value | Label | Value
                // Adjust proportions based on content to prevent text wrapping
                // Fixed column proportions so value cells (e.g., "3:00" and "YES") are the same width across rows
                // Use fixed width for left labels to prevent proportional changes when center label width changes
                let is_half_duration_row = table_row.left_label == "Half Duration";
                let is_sudden_death_or_timeout_row =
                    center_label == "Sudden Death" || center_label == "Last 2 Min Ref T/Out";
                let is_overtime_or_timeout_value_row =
                    table_row.left_label == "Overtime" || table_row.left_label == "Timeouts";
                let left_label_width = if is_half_duration_row { 120.0 } else { 90.0 }; // Increased to 90px to prevent "Timeouts" from wrapping
                let center_label_portion = if is_half_duration_row { 3 } else { 1 };

                let row_element = row![
                    container(text(table_row.left_label).size(SMALL_TEXT))
                        .padding([1, 1])
                        .width(Length::Fixed(left_label_width))
                        .style(container::rounded_box),
                    container(
                        text(table_row.left_value.clone())
                            .size(SMALL_TEXT)
                            .width(Length::Fill)
                            .align_x(Horizontal::Center)
                    )
                    .center_x(Length::Fill)
                    .padding([1, 1])
                    .width(if is_overtime_or_timeout_value_row {
                        Length::Fixed(86.0)
                    } else {
                        Length::Fixed(60.0)
                    }) // Fixed width for Overtime/Timeouts value cells
                    .style(container::rounded_box),
                    container(
                        text(center_label)
                            .size(SMALL_TEXT)
                            .align_x(Horizontal::Left)
                            .width(Length::Fill)
                    )
                    .padding([1, 1])
                    .width(if is_sudden_death_or_timeout_row {
                        Length::Fixed(180.0)
                    } else {
                        Length::FillPortion(center_label_portion)
                    })
                    .style(container::rounded_box),
                    container(
                        text(center_value.clone())
                            .size(SMALL_TEXT)
                            .width(Length::Fill)
                            .align_x(Horizontal::Center)
                    )
                    .center_x(Length::Fill)
                    .padding([1, 1])
                    .width(Length::Fixed(86.0)) // All right-side value cells same width for alignment
                    .style(container::rounded_box),
                ]
                .spacing(1)
                .width(Length::Fill);

                details_column = details_column.push(row_element);
            } else {
                // Create a 3-column row: Label | Color | TeamName (for game rows)

                // Determine font size for team names
                // Maintain same font size across the two rows of each game section
                // When we hit a header ("Last Game" or "Next Game"), update the current section
                if table_row.left_label == "Last Game" || table_row.left_label == "Next Game" {
                    current_game_header = Some(table_row.left_label.clone());
                }

                let is_game_row_color = table_row.left_value == "White" || table_row.left_value == "Black";
                let is_game_section = table_row.left_label == "Last Game"
                    || table_row.left_label == "Next Game"
                    || table_row.left_label.is_empty();

                let team_name_font_size = if is_game_row_color && is_game_section {
                    // Use the current section (Last/Next Game) for both rows
                    let header = current_game_header.as_deref().unwrap_or_else(|| {
                        if table_row.left_label == "Next Game" { "Next Game" } else { "Last Game" }
                    });
                    if header == "Next Game" {
                        dynamic_font_sizing.get_font_size(GameInfoCell::NextGame)
                    } else {
                        dynamic_font_sizing.get_font_size(GameInfoCell::LastGame)
                    }
                } else {
                    SMALL_TEXT
                };

                let row_element = row![
                    container(text(table_row.left_label).size(SMALL_TEXT))
                        .padding([1, 1])
                        .width(Length::Fixed(GAME_LABEL_WIDTH))
                        .style(container::rounded_box),
                    container(
                        text(table_row.left_value.clone())
                            .size(SMALL_TEXT)
                            .width(Length::Fill)
                            .align_x(Horizontal::Center)
                    )
                    .center_x(Length::Fill)
                    .padding([1, 1])
                    .width(Length::Fixed(60.0)) // Fixed width for color labels
                    .style(container::rounded_box),
                    container(
                        text(center_label)
                            .size(team_name_font_size)
                            .align_x(Horizontal::Left)
                            .width(Length::Fill)
                    )
                    .padding([1, 1])
                    .width(Length::Fill) // Team name takes remaining space
                    .height(Length::Fixed(GAME_INFO_CELL_HEIGHT))
                    .align_y(Vertical::Center)
                    .style(container::rounded_box),
                ]
                .spacing(1)
                .width(Length::Fill);

                details_column = details_column.push(row_element);
            }
        } else {
            // Single 2-column row: Label | Value (spanning full width)
            // Give more space to values for people's names
            let (label_portion, value_portion) = (3, 5); // Keep consistent; value column wider for names

            let label_width = match table_row.left_label.as_str() {
                "Last Game" | "Next Game" => Length::Fixed(GAME_LABEL_WIDTH),
                "" => Length::Fixed(GAME_LABEL_WIDTH), // Empty labels for second rows of game info
                "Chief Ref" | "Timer" | "Water Ref 1" | "Water Ref 2" | "Water Ref 3" => {
                    Length::Fixed(REF_LABEL_WIDTH)
                }
                _ => Length::FillPortion(label_portion),
            };

            let font_size = get_font_size_for_table_row(&table_row.left_label, dynamic_font_sizing);
            let is_ref_row_label = matches!(
                table_row.left_label.as_str(),
                "Chief Ref" | "Timer" | "Water Ref 1" | "Water Ref 2" | "Water Ref 3"
            );
            let row_element = row![
                container(text(table_row.left_label).size(SMALL_TEXT))
                    .padding([1, 2])
                    .width(label_width)
                    .style(container::rounded_box),
                {
                    // Build the value container and only lock height for Referee rows
                    let value_base = container(
                        text(table_row.left_value.clone())
                            .size(font_size)
                            .width(Length::Fill)
                            .align_x(Horizontal::Left)
                    )
                    .padding([1, 2])
                    .width(Length::FillPortion(value_portion));

                    if is_ref_row_label {
                        value_base
                            .height(Length::Fixed(GAME_INFO_CELL_HEIGHT))
                            .align_y(Vertical::Center)
                            .style(container::rounded_box)
                    } else {
                        value_base.style(container::rounded_box)
                    }
                },
            ]
            .spacing(1)
            .width(Length::Fill);

            details_column = details_column.push(row_element);
        }
    }

    container(details_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_size_mapping_preserves_alignment() {
        let dfs = DynamicFontSizing::new();

        // Test that target cell labels map correctly to dynamic font sizes
        assert_eq!(get_font_size_for_table_row("Chief Ref", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("Timer", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("Water Ref 1", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("Water Ref 2", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("Water Ref 3", &dfs), SMALL_TEXT);

        // Test that non-target labels return default font size
        assert_eq!(get_font_size_for_table_row("Last Game", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("Next Game", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("", &dfs), SMALL_TEXT); // Empty labels
        assert_eq!(get_font_size_for_table_row("Half Duration", &dfs), SMALL_TEXT);
        assert_eq!(get_font_size_for_table_row("Unknown Label", &dfs), SMALL_TEXT);
    }
}


