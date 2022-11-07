use super::{
    style::{self, BORDER_RADIUS, MIN_BUTTON_SIZE, SMALL_TEXT, SPACING},
    *,
};

use iced::{
    alignment::Horizontal,
    pure::{button, column, container, horizontal_space, row, text, vertical_space, Element},
    Alignment, Length,
};

use std::time::Duration;
use uwh_common::game_snapshot::GameSnapshot;

pub(in super::super) fn build_game_config_edit_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
) -> Element<'a, Message> {
    const NO_SELECTION_TXT: &str = "None Selected";
    const LOADING_TXT: &str = "Loading...";

    let EditableSettings {
        config,
        game_number,
        white_on_right,
        using_uwhscores,
        current_tid,
        current_pool,
        games,
    } = settings;

    let using_uwhscores = *using_uwhscores;

    let white_inner = container("WHITE")
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container::White);
    let black_inner = container("BLACK")
        .center_x()
        .center_y()
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container::Black);
    let white_spacer = container("")
        .width(Length::Units(BORDER_RADIUS.ceil() as u16))
        .height(Length::Fill)
        .style(style::Container::WhiteSharpCorner);
    let black_spacer = container("")
        .width(Length::Units(BORDER_RADIUS.ceil() as u16))
        .height(Length::Fill)
        .style(style::Container::BlackSharpCorner);

    // `white_on_right` is based on the view from the front of the panels, so for the ref's point
    // of view we need to reverse the direction
    let sides = if !white_on_right {
        // White to Ref's right
        let white_outer = container(
            row()
                .push(white_spacer)
                .push(white_inner)
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::White);
        let black_outer = container(
            row()
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16)))
                .push(black_inner)
                .push(black_spacer),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::Black);
        row().push(black_outer).push(white_outer)
    } else {
        // White to Ref's left
        let white_outer = container(
            row()
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16)))
                .push(white_inner)
                .push(white_spacer),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::White);
        let black_outer = container(
            row()
                .push(black_spacer)
                .push(black_inner)
                .push(horizontal_space(Length::Units(BORDER_RADIUS.ceil() as u16))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(style::Container::Black);
        row().push(white_outer).push(black_outer)
    };

    let sides_btn = button(sides.width(Length::Fill).height(Length::Fill))
        .height(Length::Units(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .padding(0)
        .style(style::Button::Gray)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    let rows: [Element<Message>; 4] = if using_uwhscores {
        let tournament_label = if let Some(ref tournaments) = tournaments {
            if let Some(tid) = current_tid {
                match tournaments.get(tid) {
                    Some(t) => t.name.clone(),
                    None => NO_SELECTION_TXT.to_string(),
                }
            } else {
                NO_SELECTION_TXT.to_string()
            }
        } else {
            LOADING_TXT.to_string()
        };

        let tournament_btn_msg = if tournaments.is_some() {
            Some(Message::SelectParameter(ListableParameter::Tournament))
        } else {
            None
        };

        let pool_label = if let Some(tournament) = tournaments
            .as_ref()
            .and_then(|tournaments| tournaments.get(&(*current_tid)?))
        {
            if tournament.pools.is_some() {
                if let Some(ref pool) = current_pool {
                    pool.clone()
                } else {
                    NO_SELECTION_TXT.to_string()
                }
            } else {
                LOADING_TXT.to_string()
            }
        } else {
            String::new()
        };

        let pool_btn_msg = tournaments
            .as_ref()
            .and_then(|tourns| tourns.get(&(*current_tid)?)?.pools.as_ref())
            .map(|_| Message::SelectParameter(ListableParameter::Pool));

        [
            make_value_button("TOURNAMENT:", tournament_label, true, tournament_btn_msg).into(),
            make_value_button("COURT:", pool_label, true, pool_btn_msg).into(),
            vertical_space(Length::Units(MIN_BUTTON_SIZE)).into(),
            row()
                .spacing(SPACING)
                .push(horizontal_space(Length::Fill))
                .push(horizontal_space(Length::Fill))
                .push(
                    row()
                        .spacing(SPACING)
                        .width(Length::Fill)
                        .push(
                            make_button("CANCEL")
                                .style(style::Button::Red)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: true }),
                        )
                        .push(
                            make_button("DONE")
                                .style(style::Button::Green)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: false }),
                        ),
                )
                .into(),
        ]
    } else {
        [
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "HALF LENGTH:",
                    time_string(config.half_play_duration),
                    true,
                    Some(Message::EditParameter(LengthParameter::Half)),
                ))
                .push(make_value_button(
                    "OVERTIME\nALLOWED:",
                    bool_string(config.overtime_allowed),
                    true,
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::OvertimeAllowed,
                    )),
                ))
                .push(make_value_button(
                    "SUDDEN DEATH\nALLOWED:",
                    bool_string(config.sudden_death_allowed),
                    true,
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::SuddenDeathAllowed,
                    )),
                ))
                .into(),
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "HALF TIME\nLENGTH:",
                    time_string(config.half_time_duration),
                    true,
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
                ))
                .push(make_value_button(
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    true,
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::PreOvertime))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "PRE SD\nBREAK LENGTH:",
                    time_string(config.pre_sudden_death_duration),
                    true,
                    if config.sudden_death_allowed {
                        Some(Message::EditParameter(LengthParameter::PreSuddenDeath))
                    } else {
                        None
                    },
                ))
                .into(),
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "NOMINAL BRK\nBTWN GAMES:",
                    time_string(config.nominal_break),
                    true,
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    true,
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "NUM TEAM T/Os\nALLWD PER HALF:",
                    config.team_timeouts_per_half.to_string(),
                    true,
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        config.team_timeout_duration,
                    ))),
                ))
                .into(),
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "MINIMUM BRK\nBTWN GAMES:",
                    time_string(config.minimum_break),
                    true,
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    true,
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
                    } else {
                        None
                    },
                ))
                .push(
                    row()
                        .spacing(SPACING)
                        .width(Length::Fill)
                        .push(
                            make_button("CANCEL")
                                .style(style::Button::Red)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: true }),
                        )
                        .push(
                            make_button("DONE")
                                .style(style::Button::Green)
                                .width(Length::Fill)
                                .on_press(Message::ConfigEditComplete { canceled: false }),
                        ),
                )
                .into(),
        ]
    };

    let game_btn_msg = if using_uwhscores {
        if current_tid.is_some() && current_pool.is_some() {
            Some(Message::SelectParameter(ListableParameter::Game))
        } else {
            None
        }
    } else {
        Some(Message::KeypadPage(KeypadPage::GameNumber))
    };

    let mut game_large_text = true;
    let game_label = if using_uwhscores {
        if let (Some(_), Some(cur_pool)) = (current_tid, current_pool) {
            if let Some(ref games) = games {
                match games.get(game_number) {
                    Some(game) => {
                        if game.pool == *cur_pool {
                            game_string_short(game)
                        } else {
                            game_large_text = false;
                            NO_SELECTION_TXT.to_string()
                        }
                    }
                    None => {
                        game_large_text = false;
                        NO_SELECTION_TXT.to_string()
                    }
                }
            } else {
                LOADING_TXT.to_string()
            }
        } else {
            String::new()
        }
    } else {
        game_number.to_string()
    };

    let mut col = column()
        .spacing(SPACING)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .push(make_value_button(
                    "USING UWHSCORES:",
                    bool_string(using_uwhscores),
                    true,
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::UsingUwhScores,
                    )),
                ))
                .push(sides_btn)
                .push(make_value_button(
                    "GAME:",
                    game_label,
                    game_large_text,
                    game_btn_msg,
                )),
        );

    for row in rows {
        col = col.push(row);
    }

    col.into()
}

pub fn build_game_parameter_editor<'a>(
    snapshot: &GameSnapshot,
    param: LengthParameter,
    length: Duration,
) -> Element<'a, Message> {
    let (title, hint) = match param {
        LengthParameter::Half => ("HALF LEN", "The length of a half during regular play"),
        LengthParameter::HalfTime => ("HALF TIME LEN", "The length of the Half Time period"),
        LengthParameter::NominalBetweenGame => ("NOM BREAK", "If a game runs exactly as long as scheduled, this is the length of the break between games"),
        LengthParameter::MinimumBetweenGame => ("MIN BREAK", "If a game runs longer than scheduled, this is the minimum time between games that the system will allot. If the games fall behind, the system will automatically try to catch up after subsequent games, always respecting this minimum time between games."),
        LengthParameter::PreOvertime => ("PRE OT BREAK", "If overtime is enabled and needed, this is the length of the break between Second Half and Overtime First Half"),
        LengthParameter::OvertimeHalf => ("OT HALF LEN", "The length of a half during overtime"),
        LengthParameter::OvertimeHalfTime => ("OT HLF TM LEN", "The length of Overtime Half Time"),
        LengthParameter::PreSuddenDeath => ("PRE SD BREAK", "The length of the break between the preceeding play period and Sudden Death"),
    };

    column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(make_time_editor(title, length, false))
        .push(vertical_space(Length::Fill))
        .push(
            text(String::from("Help: ") + hint)
                .size(SMALL_TEXT)
                .horizontal_alignment(Horizontal::Center),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}
