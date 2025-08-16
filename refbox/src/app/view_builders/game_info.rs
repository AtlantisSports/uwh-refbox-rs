use super::*;
use iced::{
    Length,
    widget::{column, horizontal_space, row, container},
};
use uwh_common::{
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{GameList, TeamList},
};

#[derive(Debug, Clone)]
struct TableRow {
    left_label: String,
    left_value: String,
    right_label: Option<String>,
    right_value: Option<String>,
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

    let table_rows = build_details_table(snapshot, config, using_uwhportal, games, teams);

    let mut details_column = column![]
        .spacing(SPACING / 2.0)
        .width(Length::Fill);

    for table_row in table_rows {
        if let (Some(right_label), Some(right_value)) = (&table_row.right_label, &table_row.right_value) {
            // Two-column row
            details_column = details_column.push(
                row![
                    make_value_button(
                        table_row.left_label,
                        table_row.left_value,
                        (false, false),
                        None
                    ),
                    make_value_button(
                        right_label.clone(),
                        right_value.clone(),
                        (false, false),
                        None
                    ),
                ]
                .spacing(SPACING)
                .width(Length::Fill)
            );
        } else {
            // Single-column row
            details_column = details_column.push(
                make_value_button(
                    table_row.left_label,
                    table_row.left_value,
                    (false, false),
                    None
                )
            );
        }
    }

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        container(details_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(PADDING),
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
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if using_uwhportal {
            if let Some(games) = games {
                prev_game = match games.get(&snapshot.game_number) {
                    Some(game) => game.number.to_string(),
                    None if snapshot.game_number == "0" => fl!("none").to_string(),
                    None => fl!(
                        "game-number-error",
                        game_number = snapshot.game_number.clone()
                    ),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game.number.to_string(),
                    None => fl!(
                        "next-game-number-error",
                        next_game_number = snapshot.next_game_number.clone()
                    ),
                };
            } else {
                prev_game = if snapshot.game_number == "0" {
                    fl!("none").to_string()
                } else {
                    fl!(
                        "game-number-error",
                        game_number = snapshot.game_number.clone()
                    )
                };
                next_game = fl!(
                    "next-game-number-error",
                    next_game_number = snapshot.next_game_number.clone()
                );
            }
        } else {
            prev_game = if snapshot.game_number == "0" {
                fl!("none").to_string()
            } else {
                snapshot.game_number.to_string()
            };
            next_game = snapshot.next_game_number.to_string();
        }

        table_rows.push(TableRow {
            left_label: "Last Game".to_string(),
            left_value: prev_game,
            right_label: Some("Next Game".to_string()),
            right_value: Some(next_game),
        });

        &snapshot.next_game_number
    } else {
        let game;
        if using_uwhportal {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game.number.to_string(),
                    None => fl!(
                        "game-number-error",
                        game_number = snapshot.game_number.clone()
                    ),
                };
            } else {
                game = fl!(
                    "game-number-error",
                    game_number = snapshot.game_number.clone()
                );
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        table_rows.push(TableRow {
            left_label: "Game".to_string(),
            left_value: game,
            right_label: None,
            right_value: None,
        });
        &snapshot.game_number
    };

    if using_uwhportal {
        if let Some(games) = games {
            if let Some(game) = games.get(game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                table_rows.push(TableRow {
                    left_label: "Black Team".to_string(),
                    left_value: limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    right_label: Some("White Team".to_string()),
                    right_value: Some(limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT)),
                });
            }
        }
    }

    table_rows.push(TableRow {
        left_label: "Half Duration".to_string(),
        left_value: time_string(config.half_play_duration),
        right_label: Some("Half Time Duration".to_string()),
        right_value: Some(time_string(config.half_time_duration)),
    });

    table_rows.push(TableRow {
        left_label: "Sudden Death Allowed".to_string(),
        left_value: bool_string(config.sudden_death_allowed),
        right_label: Some("Overtime Allowed".to_string()),
        right_value: Some(bool_string(config.overtime_allowed)),
    });

    if config.overtime_allowed {
        table_rows.push(TableRow {
            left_label: "Pre-Overtime Break".to_string(),
            left_value: time_string(config.pre_overtime_break),
            right_label: Some("Overtime Half Length".to_string()),
            right_value: Some(time_string(config.ot_half_play_duration)),
        });

        table_rows.push(TableRow {
            left_label: "Overtime Half Time Length".to_string(),
            left_value: time_string(config.ot_half_time_duration),
            right_label: None,
            right_value: None,
        });
    }

    if config.sudden_death_allowed {
        table_rows.push(TableRow {
            left_label: "Pre-Sudden-Death Break".to_string(),
            left_value: time_string(config.pre_sudden_death_duration),
            right_label: None,
            right_value: None,
        });
    }

    let timeout_label = if config.timeouts_counted_per_half {
        "Team Timeouts Allowed Per Half"
    } else {
        "Team Timeouts Allowed Per Game"
    };

    table_rows.push(TableRow {
        left_label: timeout_label.to_string(),
        left_value: config.num_team_timeouts_allowed.to_string(),
        right_label: None,
        right_value: None,
    });

    if config.num_team_timeouts_allowed != 0 {
        table_rows.push(TableRow {
            left_label: "Team Timeout Length".to_string(),
            left_value: time_string(config.team_timeout_duration),
            right_label: None,
            right_value: None,
        });
    }
    if !using_uwhportal {
        table_rows.push(TableRow {
            left_label: "Time Between Games".to_string(),
            left_value: time_string(config.nominal_break),
            right_label: None,
            right_value: None,
        });
    }

    table_rows.push(TableRow {
        left_label: "Minimum Break Between Games".to_string(),
        left_value: time_string(config.minimum_break),
        right_label: None,
        right_value: None,
    });

    if using_uwhportal {
        let unknown = fl!("unknown");

        table_rows.push(TableRow {
            left_label: "Stop Clock in Last 2 Minutes".to_string(),
            left_value: unknown.clone(),
            right_label: None,
            right_value: None,
        });

        table_rows.push(TableRow {
            left_label: "Chief Ref".to_string(),
            left_value: unknown.clone(),
            right_label: Some("Timer".to_string()),
            right_value: Some(unknown.clone()),
        });

        table_rows.push(TableRow {
            left_label: "Water Ref 1".to_string(),
            left_value: unknown.clone(),
            right_label: Some("Water Ref 2".to_string()),
            right_value: Some(unknown.clone()),
        });

        table_rows.push(TableRow {
            left_label: "Water Ref 3".to_string(),
            left_value: unknown,
            right_label: None,
            right_value: None,
        });
    }

    table_rows
}
