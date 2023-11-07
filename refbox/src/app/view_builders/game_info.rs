use super::{
    style::{self, SMALL_TEXT, SPACING},
    *,
};
use iced::{
    alignment::{Horizontal, Vertical},
    pure::{column, horizontal_space, row, text, Element},
    Length,
};

use std::fmt::Write;

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
    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
        ))
        .push(
            row()
                .spacing(SPACING)
                .width(Length::Fill)
                .height(Length::Fill)
                .push(
                    text(left_details)
                        .size(SMALL_TEXT)
                        .vertical_alignment(Vertical::Top)
                        .horizontal_alignment(Horizontal::Left)
                        .width(Length::Fill),
                )
                .push(
                    text(right_details)
                        .size(SMALL_TEXT)
                        .vertical_alignment(Vertical::Top)
                        .horizontal_alignment(Horizontal::Left)
                        .width(Length::Fill),
                ),
        )
        .push(
            row()
                .spacing(SPACING)
                .width(Length::Fill)
                .push(
                    make_button("BACK")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ConfigEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("SETTINGS")
                        .style(style::Button::Gray)
                        .width(Length::Fill)
                        .on_press(Message::EditGameConfig),
                ),
        )
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
                    None if snapshot.game_number == 0 => "None".to_string(),
                    None => format!("Error ({})", snapshot.game_number),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game_string_short(game),
                    None => format!("Error ({})", snapshot.next_game_number),
                };
            } else {
                prev_game = if snapshot.game_number == 0 {
                    "None".to_string()
                } else {
                    format!("Error ({})", snapshot.game_number)
                };
                next_game = format!("Error ({})", snapshot.next_game_number);
            }
        } else {
            prev_game = if snapshot.game_number == 0 {
                "None".to_string()
            } else {
                snapshot.game_number.to_string()
            };
            next_game = snapshot.next_game_number.to_string();
        }

        write!(
            &mut left_string,
            "Last Game: {}, \nNext Game: {}\n",
            prev_game, next_game
        )
        .unwrap();
        snapshot.next_game_number
    } else {
        let game;
        if using_uwhscores {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_short(game),
                    None => format!("Error ({})", snapshot.game_number),
                };
            } else {
                game = format!("Error ({})", snapshot.game_number);
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        write!(&mut left_string, "Game: {}\n", game).unwrap();
        snapshot.game_number
    };

    if using_uwhscores {
        if let Some(games) = games {
            match games.get(&game_number) {
                Some(game) => write!(
                    &mut left_string,
                    "Black Team: {}\nWhite Team: {}\n",
                    limit_team_name_len(&game.black, TEAM_NAME_LEN_LIMIT),
                    limit_team_name_len(&game.white, TEAM_NAME_LEN_LIMIT)
                )
                .unwrap(),
                None => {}
            }
        }
    }

    write!(
        &mut left_string,
        "Half Length: {}\n\
         Half Time Length: {}\n\
         Overtime Allowed: {}\n",
        time_string(config.half_play_duration),
        time_string(config.half_time_duration),
        bool_string(config.overtime_allowed),
    )
    .unwrap();
    if config.overtime_allowed {
        write!(
            &mut left_string,
            "Pre-Overtime Break Length: {}\n\
             Overtime Half Length: {}\n\
             Overtime Half Time Length: {}\n",
            time_string(config.pre_overtime_break),
            time_string(config.ot_half_play_duration),
            time_string(config.ot_half_time_duration),
        )
        .unwrap()
    } else {
    };
    write!(
        &mut left_string,
        "Sudden Death Allowed: {}\n",
        bool_string(config.sudden_death_allowed)
    )
    .unwrap();

    if config.sudden_death_allowed {
        write!(
            &mut left_string,
            "Pre-Sudden-Death Break Length: {}\n",
            time_string(config.pre_sudden_death_duration)
        )
        .unwrap()
    } else {
    };
    write!(
        &mut left_string,
        "Team Timeouts Allowed Per Half: {}\n",
        config.team_timeouts_per_half
    )
    .unwrap();
    if config.team_timeouts_per_half != 0 {
        write!(
            &mut left_string,
            "Team Timeout Duration: {}\n",
            time_string(config.team_timeout_duration)
        )
        .unwrap()
    } else {
    };
    if !using_uwhscores {
        write!(
            &mut left_string,
            "Nominal Time Between Games: {}\n",
            time_string(config.nominal_break),
        )
        .unwrap();
    }
    write!(
        &mut left_string,
        "Minimum Time Between Games: {}\n",
        time_string(config.minimum_break),
    )
    .unwrap();

    write!(&mut left_string, "Stop clock in last 2 minutes: UNKNOWN\n").unwrap();

    write!(
        &mut right_string,
        "Cheif ref: UNKNOWN\n\
        Timer: UNKNOWN\n\
        Water ref 1: UNKNOWN\n\
        Water ref 2: UNKNOWN\n\
        Water ref 3: UNKNOWN",
    )
    .unwrap();

    (left_string, right_string)
}
