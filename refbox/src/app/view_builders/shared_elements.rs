use super::{
    message::*,
    style::{
        self, BLACK, GREEN, LARGE_TEXT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, RED, SMALL_TEXT,
        SPACING, WHITE, YELLOW,
    },
};
use crate::tournament_manager::TournamentManager;
use uwh_common::{drawing_support::*, uwhscores::GameInfo};

use iced::{
    alignment::{Horizontal, Vertical},
    pure::{
        button, column, container, horizontal_space, row, text, vertical_space,
        widget::{Button, Container, Row, Text},
    },
    Alignment, Length,
};
use matrix_drawing::{secs_to_long_time_string, secs_to_time_string};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot, PenaltySnapshot, PenaltyTime, TimeoutSnapshot},
};

pub(super) fn make_scroll_list<'a, const LIST_LEN: usize>(
    buttons: [Button<'a, Message>; LIST_LEN],
    num_items: usize,
    index: usize,
    title: Text,
    scroll_option: ScrollOption,
    cont_style: impl iced::pure::widget::container::StyleSheet + 'a,
) -> Container<'a, Message> {
    let mut main_col = column().spacing(SPACING).width(Length::Fill).push(title);

    for button in buttons {
        main_col = main_col.push(button);
    }

    let top_len;
    let bottom_len;
    let can_scroll_up;
    let can_scroll_down;

    if num_items <= LIST_LEN {
        top_len = 0;
        bottom_len = 0;
        can_scroll_up = false;
        can_scroll_down = false;
    } else {
        top_len = index as u16;
        bottom_len = (num_items - LIST_LEN - index) as u16;
        can_scroll_up = index > 0;
        can_scroll_down = index + LIST_LEN < num_items;
    }

    let top_len = match top_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let bottom_len = match bottom_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let mut up_btn = make_small_button("\u{25b2}", MEDIUM_TEXT).style(style::Button::Blue);

    let mut down_btn = make_small_button("\u{25be}", MEDIUM_TEXT).style(style::Button::Blue);

    if can_scroll_up {
        up_btn = up_btn.on_press(Message::Scroll {
            which: scroll_option,
            up: true,
        });
    }

    if can_scroll_down {
        down_btn = down_btn.on_press(Message::Scroll {
            which: scroll_option,
            up: false,
        });
    }

    let scroll_bar = row()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(horizontal_space(Length::Fill))
        .push(
            container(
                column()
                    .push(vertical_space(top_len))
                    .push(
                        container(vertical_space(Length::Fill))
                            .width(Length::Fill)
                            .height(Length::FillPortion(LIST_LEN as u16))
                            .style(style::Container::Gray),
                    )
                    .push(vertical_space(bottom_len)),
            )
            .padding(PADDING)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(style::Container::LightGray),
        )
        .push(horizontal_space(Length::Fill));

    container(
        row()
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(main_col)
            .push(
                column()
                    .spacing(SPACING)
                    .width(Length::Units(MIN_BUTTON_SIZE))
                    .height(Length::Fill)
                    .push(up_btn)
                    .push(scroll_bar)
                    .push(down_btn),
            ),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(cont_style)
}

pub(in super::super) fn build_timeout_ribbon<'a>(
    snapshot: &GameSnapshot,
    tm: &Arc<Mutex<TournamentManager>>,
) -> Row<'a, Message> {
    let tm = tm.lock().unwrap();

    let black = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "BLACK\nTIMEOUT",
            tm.can_start_b_timeout()
                .ok()
                .map(|_| Message::BlackTimeout(false)),
        )
        .style(style::Button::Black),
        TimeoutSnapshot::Black(_) => make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
            .style(style::Button::Yellow),
        TimeoutSnapshot::White(_) | TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button(
                "SWITCH TO\nBLACK",
                tm.can_switch_to_b_timeout()
                    .ok()
                    .map(|_| Message::BlackTimeout(true)),
            )
            .style(style::Button::Black)
        }
    };

    let white = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "WHITE\nTIMEOUT",
            tm.can_start_w_timeout()
                .ok()
                .map(|_| Message::WhiteTimeout(false)),
        )
        .style(style::Button::White),
        TimeoutSnapshot::White(_) => make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
            .style(style::Button::Yellow),
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button(
                "SWITCH TO\nWHITE",
                tm.can_switch_to_w_timeout()
                    .ok()
                    .map(|_| Message::WhiteTimeout(true)),
            )
            .style(style::Button::White)
        }
    };

    let referee = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "REF\nTIMEOUT",
            tm.can_start_ref_timeout()
                .ok()
                .map(|_| Message::RefTimeout(false)),
        )
        .style(style::Button::Yellow),
        TimeoutSnapshot::Ref(_) => make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
            .style(style::Button::Yellow),
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::White(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button(
                "SWITCH TO\nREF",
                tm.can_switch_to_ref_timeout()
                    .ok()
                    .map(|_| Message::RefTimeout(true)),
            )
            .style(style::Button::Yellow)
        }
    };

    let penalty = match snapshot.timeout {
        TimeoutSnapshot::None => make_message_button(
            "PENALTY\nSHOT",
            tm.can_start_penalty_shot()
                .ok()
                .map(|_| Message::PenaltyShot(false)),
        )
        .style(style::Button::Red),
        TimeoutSnapshot::PenaltyShot(_) => {
            make_message_button("END\nTIMEOUT", Some(Message::EndTimeout))
                .style(style::Button::Yellow)
        }
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::White(_) | TimeoutSnapshot::Ref(_) => {
            make_message_button(
                "SWITCH TO\nPEN SHOT",
                tm.can_switch_to_penalty_shot()
                    .ok()
                    .map(|_| Message::PenaltyShot(true)),
            )
            .style(style::Button::Red)
        }
    };

    drop(tm);

    row()
        .spacing(SPACING)
        .push(black)
        .push(referee)
        .push(penalty)
        .push(white)
}

pub(super) fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    tall: bool,
    allow_red: bool,
) -> Button<'a, Message> {
    let make_red = if !allow_red {
        false
    } else {
        match snapshot.timeout {
            TimeoutSnapshot::Black(time) | TimeoutSnapshot::White(time) => {
                (time <= 10 && (time % 2 == 0) && (time != 0)) || time == 15
            }
            TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => false,
            TimeoutSnapshot::None => {
                let is_warn_period = match snapshot.current_period {
                    GamePeriod::BetweenGames
                    | GamePeriod::HalfTime
                    | GamePeriod::PreOvertime
                    | GamePeriod::OvertimeHalfTime
                    | GamePeriod::PreSuddenDeath => true,
                    GamePeriod::FirstHalf
                    | GamePeriod::SecondHalf
                    | GamePeriod::OvertimeFirstHalf
                    | GamePeriod::OvertimeSecondHalf
                    | GamePeriod::SuddenDeath => false,
                };

                snapshot.current_period != GamePeriod::SuddenDeath
                    && ((snapshot.secs_in_period <= 10
                        && (snapshot.secs_in_period % 2 == 0)
                        && (snapshot.secs_in_period != 0))
                        || (is_warn_period && snapshot.secs_in_period == 30))
            }
        }
    };

    let (mut period_text, period_color) = {
        let (text, color) = match snapshot.current_period {
            GamePeriod::BetweenGames => ("NEXT GAME", YELLOW),
            GamePeriod::FirstHalf => ("FIRST HALF", GREEN),
            GamePeriod::HalfTime => ("HALF TIME", YELLOW),
            GamePeriod::SecondHalf => ("SECOND HALF", GREEN),
            GamePeriod::PreOvertime => ("PRE OVERTIME BREAK", YELLOW),
            GamePeriod::OvertimeFirstHalf => ("OVERTIME FIRST HALF", GREEN),
            GamePeriod::OvertimeHalfTime => ("OVERTIME HALF TIME", YELLOW),
            GamePeriod::OvertimeSecondHalf => ("OVERTIME SECOND HALF", GREEN),
            GamePeriod::PreSuddenDeath => ("PRE SUDDEN DEATH BREAK", YELLOW),
            GamePeriod::SuddenDeath => ("SUDDEN DEATH", GREEN),
        };

        if make_red {
            (text, BLACK)
        } else {
            (text, color)
        }
    };

    if tall && (snapshot.timeout != TimeoutSnapshot::None) {
        match snapshot.current_period {
            GamePeriod::PreOvertime => period_text = "PRE OT BREAK",
            GamePeriod::OvertimeFirstHalf => period_text = "OT FIRST HALF",
            GamePeriod::OvertimeHalfTime => period_text = "OT HALF TIME",
            GamePeriod::OvertimeSecondHalf => period_text = "OT 2ND HALF",
            GamePeriod::PreSuddenDeath => period_text = "PRE SD BREAK",
            _ => {}
        };
    }

    macro_rules! make_time_view {
        ($base:ident, $per_text:ident, $time_text:ident) => {
            $base
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .push($per_text)
                .push($time_text)
        };
    }

    let make_time_view_row = |period_text, time_text, color| {
        let per = text(period_text)
            .color(color)
            .width(Length::Fill)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Right);
        let time = text(time_text)
            .color(color)
            .size(LARGE_TEXT)
            .width(Length::Fill)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Left);
        let r = row().spacing(SPACING);
        make_time_view!(r, per, time)
    };

    let make_time_view_col = |period_text, time_text, color| {
        let per = text(period_text).color(color);
        let time = text(time_text).color(color).size(LARGE_TEXT);
        let c = column();
        make_time_view!(c, per, time)
    };

    let mut content = row()
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center);

    let timeout_info = match snapshot.timeout {
        TimeoutSnapshot::White(_) => Some((
            if tall { "WHT TIMEOUT" } else { "WHITE TIMEOUT" },
            if make_red { BLACK } else { WHITE },
        )),
        TimeoutSnapshot::Black(_) => {
            Some((if tall { "BLK TIMEOUT" } else { "BLACK TIMEOUT" }, BLACK))
        }
        TimeoutSnapshot::Ref(_) => Some(("REF TIMEOUT", YELLOW)),
        TimeoutSnapshot::PenaltyShot(_) => Some(("PENALTY SHOT", RED)),
        TimeoutSnapshot::None => None,
    };

    let time_text = secs_to_long_time_string(snapshot.secs_in_period);
    let time_text = time_text.trim();

    if tall {
        content = content.push(make_time_view_col(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_col(
                timeout_text,
                &timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    } else {
        content = content.push(make_time_view_row(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_row(
                timeout_text,
                &timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    }

    let button_height = if tall {
        Length::Shrink
    } else {
        Length::Units(MIN_BUTTON_SIZE)
    };

    let button_style = if make_red {
        style::Button::Red
    } else {
        style::Button::Gray
    };

    button(content)
        .width(Length::Fill)
        .height(button_height)
        .padding(PADDING)
        .style(button_style)
}

pub(super) fn make_time_editor<'a, T: Into<String>>(
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'a, Message> {
    let wide = time > Duration::from_secs(MAX_STRINGABLE_SECS as u64);

    container(
        column()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(text(title).size(MEDIUM_TEXT))
            .push(
                row()
                    .spacing(SPACING)
                    .align_items(Alignment::Center)
                    .push(
                        column()
                            .spacing(SPACING)
                            .push(
                                make_small_button("+", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: true,
                                        secs: 60,
                                        timeout,
                                    }),
                            )
                            .push(
                                make_small_button("-", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: false,
                                        secs: 60,
                                        timeout,
                                    }),
                            ),
                    )
                    .push(
                        text(time_string(time))
                            .size(LARGE_TEXT)
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Units(if wide { 300 } else { 200 })),
                    )
                    .push(
                        column()
                            .spacing(SPACING)
                            .push(
                                make_small_button("+", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: true,
                                        secs: 1,
                                        timeout,
                                    }),
                            )
                            .push(
                                make_small_button("-", LARGE_TEXT)
                                    .style(style::Button::Blue)
                                    .on_press(Message::ChangeTime {
                                        increase: false,
                                        secs: 1,
                                        timeout,
                                    }),
                            ),
                    ),
            ),
    )
    .style(style::Container::LightGray)
    .padding(PADDING)
}

pub(super) fn time_string(time: Duration) -> String {
    secs_to_long_time_string(time.as_secs()).trim().to_string()
}

pub(super) fn timeout_time_string(snapshot: &GameSnapshot) -> String {
    match snapshot.timeout {
        TimeoutSnapshot::Black(secs)
        | TimeoutSnapshot::White(secs)
        | TimeoutSnapshot::Ref(secs)
        | TimeoutSnapshot::PenaltyShot(secs) => secs_to_time_string(secs).trim().to_string(),
        TimeoutSnapshot::None => String::new(),
    }
}

pub(super) fn bool_string(val: bool) -> String {
    match val {
        true => "YES".to_string(),
        false => "NO".to_string(),
    }
}

pub(super) fn penalty_string(penalties: &[PenaltySnapshot]) -> String {
    use std::fmt::Write;

    let mut string = String::new();

    for pen in penalties.iter() {
        write!(&mut string, "#{} - ", pen.player_number).unwrap();
        match pen.time {
            PenaltyTime::Seconds(secs) => {
                if secs != 0 {
                    writeln!(&mut string, "{}:{:02}", secs / 60, secs % 60).unwrap();
                } else {
                    string.push_str("Served\n");
                }
            }
            PenaltyTime::TotalDismissal => string.push_str("DSMS\n"),
        }
    }
    // if the string is not empty, the last char is a '\n' that we don't want
    string.pop();
    string
}

pub(super) fn game_string_short(game: &GameInfo) -> String {
    format!("{}{}", game.game_type, game.gid)
}

pub(super) fn game_string_long(game: &GameInfo, len_limit: usize) -> String {
    const ELIPSIS: [char; 3] = ['.', '.', '.'];

    macro_rules! limit {
        ($orig:ident) => {
            if $orig.len() > len_limit {
                Cow::Owned($orig.chars().take(len_limit - 1).chain(ELIPSIS).collect())
            } else {
                Cow::Borrowed($orig)
            }
        };
    }

    let black = &game.black;
    let black = limit!(black);
    let white = &game.white;
    let white = limit!(white);

    format!("{}{} - {} vs {}", game.game_type, game.gid, black, white)
}

pub(super) fn config_string(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
) -> String {
    const TEAM_NAME_LEN_LIMIT: usize = 12;

    let mut result = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if using_uwhscores {
            if let Some(games) = games {
                prev_game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_long(game, TEAM_NAME_LEN_LIMIT),
                    None if snapshot.game_number == 0 => "None".to_string(),
                    None => format!("Error ({})", snapshot.game_number),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game_string_long(game, TEAM_NAME_LEN_LIMIT),
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

        format!("Last Game: {}\nNext Game: {}\n", prev_game, next_game)
    } else {
        let game;
        if using_uwhscores {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_long(game, TEAM_NAME_LEN_LIMIT),
                    None => format!("Error ({})", snapshot.game_number),
                };
            } else {
                game = format!("Error ({})", snapshot.game_number);
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        format!("Game: {}\n\n", game)
    };
    result += &format!(
        "Half Length: {}\n\
         Half Time Length: {}\n\
         Overtime Allowed: {}\n",
        time_string(config.half_play_duration),
        time_string(config.half_time_duration),
        bool_string(config.overtime_allowed),
    );
    result += &if config.overtime_allowed {
        format!(
            "Pre-Overtime Break Length: {}\n\
             Overtime Half Length: {}\n\
             Overtime Half Time Length: {}\n",
            time_string(config.pre_overtime_break),
            time_string(config.ot_half_play_duration),
            time_string(config.ot_half_time_duration),
        )
    } else {
        String::new()
    };
    result += &format!(
        "Sudden Death Allowed: {}\n",
        bool_string(config.sudden_death_allowed)
    );
    result += &if config.sudden_death_allowed {
        format!(
            "Pre-Sudden-Death Break Length: {}\n",
            time_string(config.pre_sudden_death_duration)
        )
    } else {
        String::new()
    };
    result += &format!(
        "Team Timeouts Allowed Per Half: {}\n",
        config.team_timeouts_per_half
    );
    result += &if config.team_timeouts_per_half != 0 {
        format!(
            "Team Timeout Duration: {}\n",
            time_string(config.team_timeout_duration)
        )
    } else {
        String::new()
    };
    result += &format!(
        "Nominal Time Between Games: {}\n\
         Minimum Time Between Games: {}\n",
        time_string(config.nominal_break),
        time_string(config.minimum_break),
    );
    result
}

pub(super) fn make_button<'a, Message: Clone, T: Into<String>>(label: T) -> Button<'a, Message> {
    button(
        text(label)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
    )
    .padding(PADDING)
    .height(Length::Units(MIN_BUTTON_SIZE))
    .width(Length::Fill)
}

pub(super) fn make_message_button<'a, Message: Clone, T: Into<String>>(
    label: T,
    message: Option<Message>,
) -> Button<'a, Message> {
    if let Some(msg) = message {
        make_button(label).on_press(msg)
    } else {
        make_button(label)
    }
}

pub(super) fn make_small_button<'a, Message: Clone, T: Into<String>>(
    label: T,
    size: u16,
) -> Button<'a, Message> {
    button(
        text(label)
            .size(size)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fill),
    )
    .width(Length::Units(MIN_BUTTON_SIZE))
    .height(Length::Units(MIN_BUTTON_SIZE))
}

pub(super) fn make_value_button<'a, Message: 'a + Clone, T: Into<String>, U: Into<String>>(
    first_label: T,
    second_label: U,
    large_text: (bool, bool),
    message: Option<Message>,
) -> Button<'a, Message> {
    let mut button = button(
        row()
            .spacing(SPACING)
            .align_items(Alignment::Center)
            .push(
                text(first_label)
                    .size(if large_text.0 {
                        MEDIUM_TEXT
                    } else {
                        SMALL_TEXT
                    })
                    .vertical_alignment(Vertical::Center),
            )
            .push(horizontal_space(Length::Fill))
            .push(
                text(second_label)
                    .size(if large_text.1 {
                        MEDIUM_TEXT
                    } else {
                        SMALL_TEXT
                    })
                    .vertical_alignment(Vertical::Center),
            ),
    )
    .padding(PADDING)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(style::Button::LightGray);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}
