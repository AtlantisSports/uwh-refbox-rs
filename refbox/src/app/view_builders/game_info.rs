use super::*;
use iced::{
    Length,
    alignment::{Horizontal, Vertical},
    widget::{column, horizontal_space, row, text},
};
use uwh_common::{
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{Schedule, TeamList},
};

pub(in super::super) fn build_game_info_page<'a>(
    data: ViewData<'_, '_>,
    config: &GameConfig,
    using_uwhportal: bool,
    is_refreshing: bool,
    schedule: Option<&Schedule>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        portal_indicator,
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

    let (left_details, right_details) =
        details_strings(snapshot, config, using_uwhportal, schedule, teams);
    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator
        ),
        row![
            text(left_details)
                .size(SMALL_TEXT)
                .align_y(Vertical::Top)
                .align_x(Horizontal::Left)
                .width(Length::Fill),
            text(right_details)
                .size(SMALL_TEXT)
                .align_y(Vertical::Top)
                .align_x(Horizontal::Left)
                .width(Length::Fill),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
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

fn details_strings(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> (String, String) {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut right_string = String::new();
    let mut left_string = String::new();
    let games = schedule.map(|s| &s.games);
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

        left_string += &fl!(
            "last-game-next-game",
            prev_game = prev_game,
            next_game = next_game
        );

        left_string += "\n";

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
        left_string += &fl!("one-game", game = game);
        left_string += "\n";
        &snapshot.game_number
    };

    if using_uwhportal {
        if let Some(games) = games {
            if let Some(game) = games.get(game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                left_string += &fl!(
                    "black-team-white-team",
                    black_team = limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    white_team = limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT)
                );
                left_string += "\n";
            }
        }
    }

    left_string += &fl!(
        "game-length-ot-allowed",
        half_length = time_string(config.half_play_duration),
        half_time_length = time_string(config.half_time_duration),
        overtime = bool_string(config.overtime_allowed)
    );
    left_string += "\n";

    if config.overtime_allowed {
        left_string += &fl!(
            "overtime-details",
            pre_overtime = time_string(config.pre_overtime_break),
            overtime_len = time_string(config.ot_half_play_duration),
            overtime_half_time_len = time_string(config.ot_half_time_duration)
        );
        left_string += "\n";
    };

    left_string += &fl!("sd-allowed", sd = bool_string(config.sudden_death_allowed));
    left_string += "\n";

    if config.sudden_death_allowed {
        left_string += &fl!(
            "pre-sd",
            pre_sd_len = time_string(config.pre_sudden_death_duration)
        );
        left_string += "\n";
    };

    left_string += &if config.timeouts_counted_per_half {
        fl!(
            "team-timeouts-per-half",
            team_timeouts = config.num_team_timeouts_allowed
        )
    } else {
        fl!(
            "team-timeouts-per-game",
            team_timeouts = config.num_team_timeouts_allowed
        )
    };
    left_string += "\n";

    if config.num_team_timeouts_allowed != 0 {
        left_string += &fl!(
            "team-to-len",
            to_len = time_string(config.team_timeout_duration)
        );
        left_string += "\n";
    };
    if !using_uwhportal {
        left_string += &fl!(
            "time-btwn-games",
            time_btwn = time_string(config.nominal_break)
        );
        left_string += "\n";
    }
    left_string += &fl!(
        "min-brk-btwn-games",
        min_brk_time = time_string(config.minimum_break)
    );
    left_string += "\n";

    if using_uwhportal {
        let stop_clock = if let Some(sched) = schedule {
            if let Some(timing_rule) = sched.get_game_timing(game_number) {
                bool_string(timing_rule.last_2_min_stop_time)
            } else {
                fl!("unknown")
            }
        } else {
            fl!("unknown")
        };
        left_string += &fl!("stop-clock-last-2", stop_clock = stop_clock);
        left_string += "\n";

        let unknown = fl!("unknown");
        let mut chief_ref = unknown.clone();
        let mut timer = unknown.clone();
        let mut water_ref_1 = unknown.clone();
        let mut water_ref_2 = unknown.clone();
        let mut water_ref_3 = unknown.clone();
        let mut has_individual_refs = false;

        let simple_game_number = if let Some(games) = games {
            if let Some(game) = games.get(game_number) {
                if let Some(refs) = &game.referee_assignments {
                    for ref_assignment in refs {
                        if ref_assignment.user_id.is_some() {
                            has_individual_refs = true;
                            let display = ref_assignment
                                .display_name
                                .clone()
                                .unwrap_or_else(|| unknown.clone());
                            match ref_assignment.role.as_str() {
                                "Chief" => chief_ref = display,
                                "TimeOrScoreKeeper" => timer = display,
                                "Water1" => water_ref_1 = display,
                                "Water2" => water_ref_2 = display,
                                "Water3" => water_ref_3 = display,
                                _ => {}
                            }
                        }
                    }
                }
                Some(game.number.clone())
            } else {
                None
            }
        } else {
            None
        };

        if !has_individual_refs {
            if let Some(sched) = schedule {
                if let Some(refs_by_game) = &sched.referees_by_game_number {
                    let lookup_key = simple_game_number.as_ref().unwrap_or(game_number);
                    if let Some(game_refs) = refs_by_game.get(lookup_key) {
                        let ref_team = game_refs
                            .referees
                            .as_ref()
                            .and_then(|r| r.team.as_ref())
                            .and_then(|t| t.name.clone())
                            .unwrap_or_else(|| unknown.clone());

                        let ts_keeper_team = game_refs
                            .time_or_score_keeper
                            .as_ref()
                            .and_then(|r| r.team.as_ref())
                            .and_then(|t| t.name.clone())
                            .unwrap_or_else(|| unknown.clone());

                        right_string += &fl!(
                            "team-ref-list",
                            ref_team = ref_team,
                            ts_keeper_team = ts_keeper_team
                        );
                        return (left_string, right_string);
                    }
                }
            }
        }

        right_string += &fl!(
            "ref-list",
            chief_ref = chief_ref,
            timer = timer,
            water_ref_1 = water_ref_1,
            water_ref_2 = water_ref_2,
            water_ref_3 = water_ref_3
        );
    }

    (left_string, right_string)
}
