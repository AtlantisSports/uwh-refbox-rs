use super::*;
use crate::app::view_builders::shared_elements::{
    bool_string, get_team_name, limit_team_name_len, time_string,
};
use iced::{
    Element, Length,
    alignment::Horizontal,
    widget::{Space, button, column, container, horizontal_space, row, text},
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::schedule::{GameList, TeamList},
};

#[derive(Debug, Clone)]
struct TableRow {
    left_label: String,
    left_value: String,
    center_label: Option<String>,
    center_value: Option<String>,
}

pub(in super::super) fn build_game_info_page<'a>(
    data: ViewData<'_, '_>,
    config: &GameConfig,
    using_uwhportal: bool,
    is_refreshing: bool,
    games: Option<&GameList>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        ..
    } = data;

    let middle_item: Element<_> = if using_uwhportal {
        if is_refreshing {
            make_button(fl!("refreshing"))
                .style(blue_button)
                .width(Length::Fill)
                .into()
        } else {
            make_button(fl!("refresh"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press(Message::RequestPortalRefresh)
                .into()
        }
    } else {
        horizontal_space().into()
    };

    // Create a container with the table from build_details_table
    let details_table_rows = build_details_table(snapshot, config, using_uwhportal, games, teams);

    // Convert table rows to visual elements
    let mut details_column = column![]
        .spacing(SPACING / 6.0) // Reduced spacing to fit more rows
        .width(Length::Fill);

    for table_row in details_table_rows {
        let row_element = if let Some(center_label) = table_row.center_label {
            if let Some(center_value) = table_row.center_value {
                // 4-column row OR special 2-column stop clock row
                if table_row.left_label == "Stop clock during last 2 minutes of the game" {
                    // Special 2-column layout for stop clock
                    row![
                        container(
                            text(table_row.left_label)
                                .size(SMALL_TEXT)
                                .align_x(Horizontal::Left)
                        )
                        .padding([1, 1])
                        .width(Length::Fixed(388.0))
                        .style(container::rounded_box),
                        container(
                            text(center_value)
                                .size(SMALL_TEXT)
                                .width(Length::Fill)
                                .align_x(Horizontal::Center)
                        )
                        .center_x(Length::Fill)
                        .padding([1, 1])
                        .width(Length::Fixed(86.0))
                        .style(container::rounded_box),
                    ]
                } else {
                    // Regular 4-column layout
                    row![
                        container(
                            text(table_row.left_label)
                                .size(SMALL_TEXT)
                                .align_x(Horizontal::Left)
                        )
                        .padding([1, 1])
                        .width(Length::Fixed(120.0))
                        .style(container::rounded_box),
                        container(
                            text(table_row.left_value.clone())
                                .size(SMALL_TEXT)
                                .width(Length::Fill)
                                .align_x(Horizontal::Center)
                        )
                        .center_x(Length::Fill)
                        .padding([1, 1])
                        .width(Length::Fixed(86.0))
                        .style(container::rounded_box),
                        container(
                            text(center_label)
                                .size(SMALL_TEXT)
                                .align_x(Horizontal::Left)
                                .width(Length::Fill)
                        )
                        .padding([1, 1])
                        .width(Length::Fixed(180.0))
                        .style(container::rounded_box),
                        container(
                            text(center_value)
                                .size(SMALL_TEXT)
                                .width(Length::Fill)
                                .align_x(Horizontal::Center)
                        )
                        .center_x(Length::Fill)
                        .padding([1, 1])
                        .width(Length::Fixed(86.0))
                        .style(container::rounded_box),
                    ]
                }
            } else {
                // 3-column layout for game team rows
                row![
                    container(
                        text(table_row.left_label)
                            .size(SMALL_TEXT)
                            .align_x(Horizontal::Left)
                    )
                    .padding([1, 1])
                    .width(Length::Fixed(100.0))
                    .style(container::rounded_box),
                    container(
                        text(table_row.left_value.clone())
                            .size(SMALL_TEXT)
                            .width(Length::Fill)
                            .align_x(Horizontal::Center)
                    )
                    .center_x(Length::Fill)
                    .padding([1, 1])
                    .width(Length::Fixed(60.0))
                    .style(container::rounded_box),
                    container(
                        text(center_label)
                            .size(SMALL_TEXT)
                            .width(Length::Fill)
                            .align_x(Horizontal::Left)
                    )
                    .padding([1, 1])
                    .width(Length::Fill)
                    .style(container::rounded_box),
                ]
            }
        } else {
            // 2-column layout for officials
            row![
                container(
                    text(table_row.left_label)
                        .size(SMALL_TEXT)
                        .align_x(Horizontal::Left)
                )
                .padding([1, 1])
                .width(Length::Fixed(120.0))
                .style(container::rounded_box),
                container(
                    text(table_row.left_value)
                        .size(SMALL_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center)
                )
                .center_x(Length::Fill)
                .padding([1, 1])
                .width(Length::Fill)
                .style(container::rounded_box),
            ]
        };

        details_column = details_column.push(row_element.spacing(1).width(Length::Fill));
    }

    let config_table = button(details_column)
        .padding(PADDING)
        .style(light_gray_button)
        .height(Length::Fill)
        .width(Length::Fill)
        .on_press(Message::EditGameConfigPage(ConfigPage::Game)); // Navigate directly to Game Options page

    // Apply width constraint only to the table to match main page table width
    let config_table_with_spacing = row![
        Space::with_width(Length::FillPortion(1)),
        config_table.width(Length::FillPortion(2)),
        Space::with_width(Length::FillPortion(1)),
    ]
    .spacing(0)
    .width(Length::Fill);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        config_table_with_spacing,
        row![
            make_button(fl!("back"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            middle_item,
            make_button(fl!("settings"))
                .style(gray_button)
                .width(Length::Fill)
                .on_press(Message::EditGameConfig),
        ]
        .spacing(SPACING)
        .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn build_details_table(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    teams: Option<&TeamList>,
) -> Vec<TableRow> {
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

    // Row: Timeouts | 1 / Game | Timeout Duration | 2:00
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
        center_label: Some("Timeout Duration".to_string()),
        center_value: Some(time_string(config.team_timeout_duration)),
    });

    // Row: Stop clock during last 2 minutes of the game | empty | empty | YES
    table_rows.push(TableRow {
        left_label: "Stop clock during last 2 minutes of the game".to_string(),
        left_value: "".to_string(), // Empty left value to make space for the label
        center_label: Some("".to_string()), // Empty center label
        center_value: Some("YES".to_string()), // YES aligns with timeout duration above
    });

    // Row: Overtime | YES | Pre-Overtime Break | 5:00
    table_rows.push(TableRow {
        left_label: "Overtime".to_string(),
        left_value: bool_string(config.overtime_allowed),
        center_label: Some("Pre-Overtime Break".to_string()),
        center_value: Some(time_string(config.pre_overtime_break)),
    });

    // Row: Sudden Death | YES | Pre-Sudden Death | 2:00
    table_rows.push(TableRow {
        left_label: "Sudden Death".to_string(),
        left_value: bool_string(config.sudden_death_allowed),
        center_label: Some("Pre-Sudden Death".to_string()),
        center_value: Some(time_string(config.pre_sudden_death_duration)),
    });

    // Officials information - compact layout to match screenshot
    let unknown = fl!("unknown");

    // Row 5: Chief Ref | Unknown (single column)
    table_rows.push(TableRow {
        left_label: "Chief Ref".to_string(),
        left_value: unknown.clone(),
        center_label: None,
        center_value: None,
    });

    // Row 6: Timer | Unknown (single column)
    table_rows.push(TableRow {
        left_label: "Timer".to_string(),
        left_value: unknown.clone(),
        center_label: None,
        center_value: None,
    });

    // Row 7: Water Ref 1 | Unknown (single column)
    table_rows.push(TableRow {
        left_label: "Water Ref 1".to_string(),
        left_value: unknown.clone(),
        center_label: None,
        center_value: None,
    });

    // Row 8: Water Ref 2 | Unknown (single column)
    table_rows.push(TableRow {
        left_label: "Water Ref 2".to_string(),
        left_value: unknown.clone(),
        center_label: None,
        center_value: None,
    });

    // Row 9: Water Ref 3 | Unknown (single column)
    table_rows.push(TableRow {
        left_label: "Water Ref 3".to_string(),
        left_value: unknown.clone(),
        center_label: None,
        center_value: None,
    });

    table_rows
}
