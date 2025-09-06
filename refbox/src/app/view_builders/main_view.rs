use super::*;
use crate::app::theme::{
    team_color_black_container_square, team_color_white_container_square,
};
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

// Constants for fixed-width label cells
const GAME_LABEL_WIDTH: f32 = 120.0;
const REF_LABEL_WIDTH: f32 = 120.0;
// Fixed cell height to keep rows from shrinking when font sizes are reduced
const GAME_INFO_CELL_HEIGHT: f32 = 26.0;

#[derive(Debug, Clone)]
struct TableRow {
    left_label: String,
    left_value: String,
    center_label: Option<String>,
    center_value: Option<String>,
    score: Option<String>, // New field for score display
}

pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    track_fouls_and_warnings: bool,
    last_game_scores: Option<(u8, u8)>, // (black, white) scores from previous game
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        font_demo: _,
        demo_data_type: _,
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
            last_game_scores,
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
    last_game_scores: Option<(u8, u8)>, // (black, white) scores from previous game
) -> Element<'a, Message> {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut table_rows = Vec::new();

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
    let (prev_black_team, prev_white_team) = if using_uwhportal {
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

    // Determine labels based on game state:
    // - During NEXT GAME state (between games): "Prior Game" and "Next Game"
    // - During active gameplay: "Current" and "Next Game"
    let (last_label, next_label) = if snapshot.current_period == GamePeriod::BetweenGames {
        (fl!("prior-game"), fl!("next-game"))
    } else {
        (fl!("current-game"), fl!("next-game"))
    };

    // Determine what scores to show based on game state and available data
    let (white_score, black_score) = if snapshot.current_period == GamePeriod::BetweenGames {
        // Between games - show previous game final scores if available, otherwise use "-"
        if let Some((prev_white, prev_black)) = last_game_scores {
            (Some(prev_white.to_string()), Some(prev_black.to_string()))
        } else {
            (Some("-".to_string()), Some("-".to_string()))
        }
    } else {
        // During active game - show current scores
        (Some(snapshot.scores.white.to_string()), Some(snapshot.scores.black.to_string()))
    };

    // Add "Last" rows - first row with White team
    table_rows.push(TableRow {
        left_label: last_label.clone(),
        left_value: "White".to_string(),
        center_label: Some(prev_white_team.clone()),
        center_value: None,
        score: white_score,
    });

    // Add second row for Last with Black team (empty label)
    table_rows.push(TableRow {
        left_label: "".to_string(),
        left_value: "Black".to_string(),
        center_label: Some(prev_black_team.clone()),
        center_value: None,
        score: black_score,
    });

    // Add "Next" rows - first row with White team
    table_rows.push(TableRow {
        left_label: next_label.clone(),
        left_value: "White".to_string(),
        center_label: Some(current_white_team.clone()),
        center_value: None,
        score: None,
    });

    // Add second row for Next with Black team (empty label)
    table_rows.push(TableRow {
        left_label: "".to_string(),
        left_value: "Black".to_string(),
        center_label: Some(current_black_team.clone()),
        center_value: None,
        score: None,
    });

    // Compact layout - everything in fewer rows to match screenshot
    // Row: Half Duration | 15:00 | Half Time Duration | 3:00
    table_rows.push(TableRow {
        left_label: "Half Duration".to_string(),
        left_value: time_string(config.half_play_duration),
        center_label: Some("Half Time Duration".to_string()),
        center_value: Some(time_string(config.half_time_duration)),
        score: None,
    });

    // Row: Timeouts | 1 / Game | Last 2 Min Ref T/Out | YES
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
        center_label: Some("Last 2 Min Stop Clock".to_string()),
        center_value: Some("YES".to_string()), // Default value, will be configurable later
        score: None,
    });

    // Row: Overtime | YES | Sudden Death | YES
    table_rows.push(TableRow {
        left_label: "Overtime".to_string(),
        left_value: bool_string(config.overtime_allowed),
        center_label: Some("Sudden Death".to_string()),
        center_value: Some(bool_string(config.sudden_death_allowed)),
        score: None,
    });

    // Officials information - compact layout to match screenshot
    if !fouls_and_warnings {
        let unknown = fl!("unknown");

        // Row 5: Chief Ref | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Chief Ref".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            score: None,
        });

        // Row 6: Timer | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Timer".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            score: None,
        });

        // Row 7: Water Ref 1 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 1".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            score: None,
        });

        // Row 8: Water Ref 2 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 2".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            score: None,
        });

        // Row 9: Water Ref 3 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 3".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            score: None,
        });
    }

    // Build the table layout with compact spacing to fit all content
    let mut details_column = column![]
        .spacing(SPACING / 4.0) // Much smaller spacing to fit more rows
        .width(Length::Fill);

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
                    center_label == "Sudden Death" || center_label == "Last 2 Min Stop Clock";
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
                // Create a row for team names - always 3-column layout with score cell for team rows
                let is_team_row = table_row.left_value == "White" || table_row.left_value == "Black";
                
                if is_team_row && table_row.score.is_some() {
                    // 3-column row for team with score: Label | TeamName | Score
                    let score_text = table_row.score.as_ref().unwrap().clone();
                    let row_element = row![
                        container(text(table_row.left_label).size(SMALL_TEXT))
                            .padding([1, 1])
                            .width(Length::Fixed(120.0))
                            .style(container::rounded_box),
                        container(
                            text(center_label)
                                .size(SMALL_TEXT)
                                .align_x(Horizontal::Left)
                                .width(Length::Fill)
                        )
                        .padding([1, 1])
                        .width(Length::Fill) // Team name takes most of the remaining space
                        .height(Length::Fixed(GAME_INFO_CELL_HEIGHT))
                        .align_y(Vertical::Center)
                        .style(if table_row.left_value == "White" {
                            team_color_white_container_square
                        } else {
                            team_color_black_container_square
                        }),
                        container(
                            text(score_text)
                                .size(SMALL_TEXT)
                                .align_x(Horizontal::Center)
                                .width(Length::Fill)
                        )
                        .padding([1, 1])
                        .width(Length::Fixed(40.0)) // Square-ish score cell
                        .height(Length::Fixed(GAME_INFO_CELL_HEIGHT))
                        .align_y(Vertical::Center)
                        .style(if table_row.left_value == "White" {
                            team_color_white_container_square
                        } else {
                            team_color_black_container_square
                        }),
                    ]
                    .spacing(1)
                    .width(Length::Fill);

                    details_column = details_column.push(row_element);
                } else {
                    // Create a 2-column row: Label | TeamName (with colored background for game rows)

                    let row_element = row![
                        container(text(table_row.left_label).size(SMALL_TEXT))
                            .padding([1, 1])
                            .width(Length::Fixed(120.0)) // Wider to absorb the former color indicator space
                            .style(container::rounded_box),
                        container(
                            text(center_label)
                                .size(SMALL_TEXT)
                                .align_x(Horizontal::Left)
                                .width(Length::Fill)
                        )
                        .padding([1, 1])
                        .width(Length::Fill) // Team name takes remaining space with colored background
                        .height(Length::Fixed(GAME_INFO_CELL_HEIGHT))
                        .align_y(Vertical::Center)
                        .style(if table_row.left_value == "White" {
                            team_color_white_container_square
                        } else if table_row.left_value == "Black" {
                            team_color_black_container_square
                        } else {
                            container::rounded_box
                        }),
                    ]
                    .spacing(1)
                    .width(Length::Fill);

                    details_column = details_column.push(row_element);
                }
            }
        } else {
            // Single 2-column row: Label | Value (spanning full width)
            // Give more space to values for people's names
            let (label_portion, value_portion) = (3, 5); // Keep consistent; value column wider for names

            let label_width = match table_row.left_label.as_str() {
                // Game-related labels (both static and dynamic)
                "Last" | "Next" => Length::Fixed(GAME_LABEL_WIDTH),
                label if label == fl!("prior-game") || label == fl!("current-game") || label == fl!("next-game") => Length::Fixed(GAME_LABEL_WIDTH),
                "" => Length::Fixed(GAME_LABEL_WIDTH), // Empty labels for second rows of game info
                "Chief Ref" | "Timer" | "Water Ref 1" | "Water Ref 2" | "Water Ref 3" => {
                    Length::Fixed(REF_LABEL_WIDTH)
                }
                _ => Length::FillPortion(label_portion),
            };

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
                            .size(SMALL_TEXT)
                            .width(Length::Fill)
                            .align_x(Horizontal::Left),
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
