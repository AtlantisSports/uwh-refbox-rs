use super::{
    fl,
    style::{ButtonStyle, Element, SMALL_TEXT, SPACING},
    *,
};
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{column, horizontal_space, row, text},
    Length,
};

use uwh_common::game_snapshot::GameSnapshot;

pub(in super::super) fn build_game_info_page<'a>(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let (left_details, right_details) = details_strings(snapshot, config, using_uwhscores, games);
    column![
        make_game_time_button(snapshot, false, false, mode, clock_running,),
        row![
            text(left_details)
                .size(SMALL_TEXT)
                .vertical_alignment(Vertical::Top)
                .horizontal_alignment(Horizontal::Left)
                .width(Length::Fill),
            text(right_details)
                .size(SMALL_TEXT)
                .vertical_alignment(Vertical::Top)
                .horizontal_alignment(Horizontal::Left)
                .width(Length::Fill),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
        row![
            make_button(fl!("back"))
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button(fl!("settings"))
                .style(ButtonStyle::Gray)
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
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
) -> (String, String) {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut left_string = String::new();
    let mut right_string = String::new();
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if using_uwhscores {
            if let Some(games) = games {
                prev_game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_short(game),
                    None if snapshot.game_number == 0 => fl!("none").to_string(),
                    None => fl!("game-number-error", game_number = snapshot.game_number),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game_string_short(game),
                    None => fl!(
                        "next-game-number-error",
                        next_game_number = snapshot.next_game_number
                    ),
                };
            } else {
                prev_game = if snapshot.game_number == 0 {
                    fl!("none").to_string()
                } else {
                    fl!("game-number-error", game_number = snapshot.game_number)
                };
                next_game = fl!(
                    "next-game-number-error",
                    next_game_number = snapshot.next_game_number
                );
            }
        } else {
            prev_game = if snapshot.game_number == 0 {
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

        snapshot.next_game_number
    } else {
        let game;
        if using_uwhscores {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_short(game),
                    None => fl!("game-number-error", game_number = snapshot.game_number),
                };
            } else {
                game = fl!("game-number-error", game_number = snapshot.game_number);
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        left_string += &fl!("one-game", game = game);
        left_string += "\n";
        snapshot.game_number
    };

    if using_uwhscores {
        if let Some(games) = games {
            if let Some(game) = games.get(&game_number) {
                left_string += &fl!(
                    "black-team-white-team",
                    black_team = limit_team_name_len(&game.black, TEAM_NAME_LEN_LIMIT),
                    white_team = limit_team_name_len(&game.white, TEAM_NAME_LEN_LIMIT)
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

    left_string += &fl!("team-tos-per-half", to_num = config.team_timeouts_per_half);
    left_string += "\n";

    if config.team_timeouts_per_half != 0 {
        left_string += &fl!(
            "team-to-len",
            to_len = time_string(config.team_timeout_duration)
        );
        left_string += "\n";
    };
    if !using_uwhscores {
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

    left_string += &fl!("stop-clock-last-2-min");
    left_string += "\n";

    right_string += &fl!("refs");

    (left_string, right_string)
}
