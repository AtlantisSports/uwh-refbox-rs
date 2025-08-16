use super::*;
use iced::{
    Alignment, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, row, text, container},
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot, PenaltyTime},
    uwhportal::schedule::{GameList, TeamList},
};

#[derive(Debug, Clone)]
struct TableRow {
    left_label: String,
    left_value: String,
    center_label: Option<String>,
    center_value: Option<String>,
    right_label: Option<String>,
    right_value: Option<String>,
}

pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    track_fouls_and_warnings: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
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
        button(
            build_config_table(
                snapshot,
                game_config,
                using_uwhportal,
                games,
                teams,
                track_fouls_and_warnings,
            )
        )
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
) -> Element<'a, Message> {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut table_rows = Vec::new();

    // Always show Game Number as Row 1
    let game_display = if using_uwhportal {
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

    // Add "Last Game" row first
    table_rows.push(TableRow {
        left_label: "Last Game".to_string(),
        left_value: "None".to_string(),
        center_label: None,
        center_value: None,
        right_label: None,
        right_value: None,
    });

    // Change "Game" to "Next Game"
    table_rows.push(TableRow {
        left_label: "Next Game".to_string(),
        left_value: game_display,
        center_label: None,
        center_value: None,
        right_label: None,
        right_value: None,
    });

    // Game number information for reference
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        &snapshot.next_game_number
    } else {
        &snapshot.game_number
    };

    // Team information
    if using_uwhportal {
        if let Some(games) = games {
            if let Some(game) = games.get(game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                table_rows.push(TableRow {
                    left_label: "Black Team".to_string(),
                    left_value: limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    center_label: Some("White Team".to_string()),
                    center_value: Some(limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT)),
                    right_label: None,
                    right_value: None,
                });
            }
        }
    }

    // Compact layout - everything in fewer rows to match screenshot
    // Row: Half Duration | 15:00 | Half Time Duration | 3:00
    table_rows.push(TableRow {
        left_label: "Half Duration".to_string(),
        left_value: time_string(config.half_play_duration),
        center_label: Some("Half Time Duration".to_string()),
        center_value: Some(time_string(config.half_time_duration)),
        right_label: None,
        right_value: None,
    });

    // Row: Overtime | YES | Sudden Death | YES (swapped positions)
    table_rows.push(TableRow {
        left_label: "Overtime".to_string(),
        left_value: bool_string(config.overtime_allowed),
        center_label: Some("Sudden Death".to_string()),
        center_value: Some(bool_string(config.sudden_death_allowed)),
        right_label: None,
        right_value: None,
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
        center_label: Some("Fin 2 Min Stop Time".to_string()),
        center_value: Some("YES".to_string()), // Default value, will be configurable later
        right_label: None,
        right_value: None,
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
            right_label: None,
            right_value: None,
        });

        // Row 6: Timer | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Timer".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            right_label: None,
            right_value: None,
        });

        // Row 7: Water Ref 1 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 1".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            right_label: None,
            right_value: None,
        });

        // Row 8: Water Ref 2 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 2".to_string(),
            left_value: unknown.clone(),
            center_label: None,
            center_value: None,
            right_label: None,
            right_value: None,
        });

        // Row 9: Water Ref 3 | Unknown (single column)
        table_rows.push(TableRow {
            left_label: "Water Ref 3".to_string(),
            left_value: unknown,
            center_label: None,
            center_value: None,
            right_label: None,
            right_value: None,
        });
    }

    // Build the table layout with compact spacing to fit all content
    let mut details_column = column![]
        .spacing(SPACING / 4.0)  // Much smaller spacing to fit more rows
        .width(Length::Fill);

    for table_row in table_rows {
        // Check if we have a center column to create a 4-column row (Label|Value|Label|Value)
        if let (Some(center_label), Some(center_value)) = (table_row.center_label, table_row.center_value) {
            // Create a 4-column row: Label | Value | Label | Value
            // Adjust proportions based on content to prevent text wrapping
            let (left_label_portion, center_label_portion) = if table_row.left_label == "Half Duration" {
                // Half Duration row: "Half Duration" (13 chars) needs more space, "Half Time Duration" (18 chars) needs most space
                (2, 3)
            } else if table_row.left_label == "Overtime" {
                // Overtime row: "Overtime" (8 chars) needs less space, "Sudden Death" (12 chars) needs more space
                (1, 2)
            } else if table_row.left_label == "Timeouts" {
                // Timeouts row: Use same proportions as Overtime row to match label column width
                (1, 2)
            } else {
                // Default proportions for any other 4-column rows
                (2, 2)
            };

            let row_element = row![
                container(text(table_row.left_label).size(SMALL_TEXT))
                    .padding([2, 1])
                    .width(Length::FillPortion(left_label_portion))
                    .style(container::rounded_box),
                container(
                    text(table_row.left_value.clone())
                        .size(SMALL_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center)
                )
                    .padding([2, 1])
                    .width(Length::FillPortion(1))  // Space for values like "15:00" or "YES"
                    .style(container::rounded_box),
                container(text(center_label).size(SMALL_TEXT))
                    .padding([2, 1])
                    .width(Length::FillPortion(center_label_portion))
                    .style(container::rounded_box),
                container(
                    text(center_value.clone())
                        .size(SMALL_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center)
                )
                    .padding([2, 1])
                    .width(Length::FillPortion(1))  // Space for values like "3:00" or "YES"
                    .style(container::rounded_box),
            ]
            .spacing(1)
            .width(Length::Fill);

            details_column = details_column.push(row_element);
        } else {
            // Single 2-column row: Label | Value (spanning full width)
            // Give more space to values for people's names
            let (label_portion, value_portion) = (3, 5);  // More space for people's names

            let row_element = row![
                container(text(table_row.left_label).size(SMALL_TEXT))
                    .padding([2, 4])
                    .width(Length::FillPortion(label_portion))
                    .style(container::rounded_box),
                container(
                    text(table_row.left_value.clone())
                        .size(SMALL_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center)
                )
                    .padding([2, 4])
                    .width(Length::FillPortion(value_portion))
                    .style(container::rounded_box),
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

fn make_value_button<'a>(label: String, value: String) -> Element<'a, Message> {
    let content = column![
        text(label).size(SMALL_TEXT * 0.8),
        text(value).size(SMALL_TEXT),
    ]
    .spacing(2)
    .width(Length::Fill);

    container(content)
        .padding([4, 8])
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn make_label_value_pair<'a>(label: String, value: String) -> Element<'a, Message> {
    // Create a 2-column layout: label on left, value on right
    row![
        container(text(label).size(SMALL_TEXT))
            .padding([4, 8])
            .width(Length::FillPortion(1))
            .style(container::rounded_box),
        container(text(value).size(SMALL_TEXT))
            .padding([4, 8])
            .width(Length::FillPortion(1))
            .style(container::rounded_box),
    ]
    .spacing(2)
    .width(Length::Fill)
    .into()
}
