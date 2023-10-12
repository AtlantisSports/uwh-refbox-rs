use super::{
    style::{self, PADDING, SMALL_TEXT, SPACING},
    *,
};
use iced::{
    alignment::{Horizontal, Vertical},
    pure::{button, column, horizontal_space, row, text, Element},
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
    let details = details_string(snapshot, config, using_uwhscores, games);
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
            button(
                text(details)
                    .size(SMALL_TEXT)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Left),
            )
            .padding(PADDING)
            .style(style::Button::LightGray)
            .width(Length::Fill)
            .height(Length::Fill),
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

fn details_string(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
) -> String {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut result = String::new();
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
            &mut result,
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
        write!(&mut result, "Game: {}\n", game).unwrap();
        snapshot.game_number
    };

    if using_uwhscores {
        if let Some(games) = games {
            match games.get(&game_number) {
                Some(game) => write!(
                    &mut result,
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
        &mut result,
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
            &mut result,
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
        &mut result,
        "Sudden Death Allowed: {}\n",
        bool_string(config.sudden_death_allowed)
    )
    .unwrap();

    if config.sudden_death_allowed {
        write!(
            &mut result,
            "Pre-Sudden-Death Break Length: {}\n",
            time_string(config.pre_sudden_death_duration)
        )
        .unwrap()
    } else {
    };
    write!(
        &mut result,
        "Team Timeouts Allowed Per Half: {}\n",
        config.team_timeouts_per_half
    )
    .unwrap();
    if config.team_timeouts_per_half != 0 {
        write!(
            &mut result,
            "Team Timeout Duration: {}\n",
            time_string(config.team_timeout_duration)
        )
        .unwrap()
    } else {
    };
    if !using_uwhscores {
        write!(
            &mut result,
            "Nominal Time Between Games: {}\n",
            time_string(config.nominal_break),
        )
        .unwrap();
    }
    write!(
        &mut result,
        "Minimum Time Between Games: {}\n",
        time_string(config.minimum_break),
    )
    .unwrap();

    write!(&mut result, "Stop clock in last 2 minutes: \n").unwrap();

    write!(
        &mut result,
        "Cheif ref: \n\
        Timer: \n\
        Water ref 1: \n\
        Water ref 2: \n\
        Water ref 3: ",
    )
    .unwrap();

    result
}
