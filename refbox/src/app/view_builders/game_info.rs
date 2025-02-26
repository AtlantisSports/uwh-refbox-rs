use super::{
    style::{ButtonStyle, Element, SMALL_TEXT, SPACING},
    *,
};
use iced::{
    Length,
    alignment::{Horizontal, Vertical},
    widget::{column, horizontal_space, row, text},
};
use std::fmt::Write;
use uwh_common::{
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{GameList, TeamList},
};

pub(in super::super) fn build_game_info_page<'a>(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    teams: Option<&TeamList>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let (left_details, right_details) =
        details_strings(snapshot, config, using_uwhportal, games, teams);
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
            make_button("BACK")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button("SETTINGS")
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
    using_uwhportal: bool,
    games: Option<&GameList>,
    teams: Option<&TeamList>,
) -> (String, String) {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let mut right_string = String::new();
    let (result_string, _) = config_string_game_num(snapshot, using_uwhportal, games);
    let mut left_string = result_string;
    let (_, result_u32) = config_string_game_num(snapshot, using_uwhportal, games);
    let game_number = result_u32;

    if using_uwhportal {
        if let (Some(games), Some(teams)) = (games, teams) {
            if let Some(game) = games.get(&game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                write!(
                    &mut left_string,
                    "Black Team: {}\nWhite Team: {}\n",
                    limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT)
                )
                .unwrap()
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
    };
    writeln!(
        &mut left_string,
        "Sudden Death Allowed: {}",
        bool_string(config.sudden_death_allowed)
    )
    .unwrap();

    if config.sudden_death_allowed {
        writeln!(
            &mut left_string,
            "Pre-Sudden-Death Break Length: {}",
            time_string(config.pre_sudden_death_duration)
        )
        .unwrap()
    };
    writeln!(
        &mut left_string,
        "Team Timeouts Allowed Per Half: {}",
        config.num_team_timeouts_allowed
    )
    .unwrap();
    if config.num_team_timeouts_allowed != 0 {
        writeln!(
            &mut left_string,
            "Team Timeout Duration: {}",
            time_string(config.team_timeout_duration)
        )
        .unwrap()
    };
    if !using_uwhportal {
        writeln!(
            &mut left_string,
            "Nominal Time Between Games: {}",
            time_string(config.nominal_break),
        )
        .unwrap();
    }
    writeln!(
        &mut left_string,
        "Minimum Time Between Games: {}",
        time_string(config.minimum_break),
    )
    .unwrap();

    if using_uwhportal {
        writeln!(&mut left_string, "Stop clock in last 2 minutes: UNKNOWN").unwrap();
    }

    if using_uwhportal {
        write!(
            &mut right_string,
            "Chief ref: UNKNOWN\n\
        Timer: UNKNOWN\n\
        Water ref 1: UNKNOWN\n\
        Water ref 2: UNKNOWN\n\
        Water ref 3: UNKNOWN",
        )
        .unwrap();
    }

    (left_string, right_string)
}
