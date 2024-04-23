use super::{
    message::*,
    style::{
        Button, ButtonStyle, Container, ContainerStyle, Row, SvgStyle, Text, TextStyle, LARGE_TEXT,
        LINE_HEIGHT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, SMALL_PLUS_TEXT, SMALL_TEXT, SPACING,
    },
    Element,
};
use crate::{config::Mode, tournament_manager::TournamentManager};
use enum_iterator::all;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        button, column, container, horizontal_space, row,
        svg::{self, Svg},
        text, vertical_space,
    },
    Alignment, Length,
};
use matrix_drawing::{secs_to_long_time_string, secs_to_time_string};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{
        Color as GameColor, GamePeriod, GameSnapshot, Infraction, InfractionSnapshot,
        PenaltySnapshot, PenaltyTime, TimeoutSnapshot,
    },
    uwhscores::GameInfo,
};

macro_rules! column {
    () => (
        iced::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Column::with_children(vec![$($crate::app::Element::from($x)),+])
    );
}

macro_rules! row {
    () => (
        iced::widget::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Row::with_children(vec![$($crate::app::Element::from($x)),+])
    );
}

pub(super) fn make_scroll_list<'a, const LIST_LEN: usize>(
    buttons: [Element<'a, Message>; LIST_LEN],
    num_items: usize,
    index: usize,
    title: Text<'a>,
    scroll_option: ScrollOption,
    cont_style: ContainerStyle,
) -> Container<'a, Message> {
    let mut main_col = column![title].spacing(SPACING).width(Length::Fill);

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

    let scroll_btn_style = if cont_style == ContainerStyle::Blue {
        ButtonStyle::BlueWithBorder
    } else {
        ButtonStyle::Blue
    };

    let mut up_btn = button(
        container(
            Svg::new(svg::Handle::from_memory(
                &include_bytes!("../../../resources/arrow_drop_up.svg")[..],
            ))
            .style(SvgStyle::White),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ContainerStyle::Transparent)
        .center_x()
        .center_y(),
    )
    .width(Length::Fixed(MIN_BUTTON_SIZE))
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(scroll_btn_style);

    let mut down_btn = button(
        container(
            Svg::new(svg::Handle::from_memory(
                &include_bytes!("../../../resources/arrow_drop_down.svg")[..],
            ))
            .style(SvgStyle::White),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ContainerStyle::Transparent)
        .center_x()
        .center_y(),
    )
    .width(Length::Fixed(MIN_BUTTON_SIZE))
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(scroll_btn_style);

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

    let scroll_bar = row![]
        .width(Length::Fill)
        .height(Length::Fill)
        .push(horizontal_space(Length::Fill))
        .push(
            container(column![
                vertical_space(top_len),
                container(vertical_space(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::FillPortion(LIST_LEN as u16))
                    .style(ContainerStyle::Gray),
                vertical_space(bottom_len),
            ])
            .padding(PADDING)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(ContainerStyle::ScrollBar),
        )
        .push(horizontal_space(Length::Fill));

    container(
        row![
            main_col,
            column![up_btn, scroll_bar, down_btn]
                .spacing(SPACING)
                .width(Length::Fixed(MIN_BUTTON_SIZE))
                .height(Length::Fill),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(cont_style)
}

pub(in super::super) fn build_timeout_ribbon<'a>(
    snapshot: &GameSnapshot,
    tm: &Arc<Mutex<TournamentManager>>,
    mode: Mode,
) -> Row<'a, Message> {
    let tm = tm.lock().unwrap();

    let black = match snapshot.timeout {
        TimeoutSnapshot::None => make_multi_label_message_button(
            ("BLACK", "TIMEOUT"),
            tm.can_start_team_timeout(GameColor::Black)
                .ok()
                .map(|_| Message::TeamTimeout(GameColor::Black, false)),
        )
        .style(ButtonStyle::Black),
        TimeoutSnapshot::Black(_) => {
            make_multi_label_message_button(("END", "TIMEOUT"), Some(Message::EndTimeout))
                .style(ButtonStyle::Yellow)
        }
        TimeoutSnapshot::White(_) | TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_multi_label_message_button(
                ("SWITCH TO", "BLACK"),
                tm.can_switch_to_team_timeout(GameColor::Black)
                    .ok()
                    .map(|_| Message::TeamTimeout(GameColor::Black, true)),
            )
            .style(ButtonStyle::Black)
        }
    };

    let white = match snapshot.timeout {
        TimeoutSnapshot::None => make_multi_label_message_button(
            ("WHITE", "TIMEOUT"),
            tm.can_start_team_timeout(GameColor::White)
                .ok()
                .map(|_| Message::TeamTimeout(GameColor::White, false)),
        )
        .style(ButtonStyle::White),
        TimeoutSnapshot::White(_) => {
            make_multi_label_message_button(("END", "TIMEOUT"), Some(Message::EndTimeout))
                .style(ButtonStyle::Yellow)
        }
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_multi_label_message_button(
                ("SWITCH TO", "WHITE"),
                tm.can_switch_to_team_timeout(GameColor::White)
                    .ok()
                    .map(|_| Message::TeamTimeout(GameColor::White, true)),
            )
            .style(ButtonStyle::White)
        }
    };

    let referee = match snapshot.timeout {
        TimeoutSnapshot::None => make_multi_label_message_button(
            ("REF", "TIMEOUT"),
            tm.can_start_ref_timeout()
                .ok()
                .map(|_| Message::RefTimeout(false)),
        )
        .style(ButtonStyle::Yellow),
        TimeoutSnapshot::Ref(_) => {
            make_multi_label_message_button(("END", "TIMEOUT"), Some(Message::EndTimeout))
                .style(ButtonStyle::Yellow)
        }
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::White(_) | TimeoutSnapshot::PenaltyShot(_) => {
            make_multi_label_message_button(
                ("SWITCH TO", "REF"),
                tm.can_switch_to_ref_timeout()
                    .ok()
                    .map(|_| Message::RefTimeout(true)),
            )
            .style(ButtonStyle::Yellow)
        }
    };

    let penalty = match snapshot.timeout {
        TimeoutSnapshot::None => make_multi_label_message_button(
            ("PENALTY", "SHOT"),
            tm.can_start_penalty_shot()
                .ok()
                .map(|_| Message::PenaltyShot(false)),
        )
        .style(ButtonStyle::Red),
        TimeoutSnapshot::PenaltyShot(_) => {
            make_multi_label_message_button(("END", "TIMEOUT"), Some(Message::EndTimeout))
                .style(ButtonStyle::Yellow)
        }
        TimeoutSnapshot::Black(_) | TimeoutSnapshot::White(_) | TimeoutSnapshot::Ref(_) => {
            let can_switch = if mode == Mode::Rugby {
                tm.can_switch_to_rugby_penalty_shot()
            } else {
                tm.can_switch_to_penalty_shot()
            };
            make_multi_label_message_button(
                ("SWITCH TO", "PEN SHOT"),
                can_switch.ok().map(|_| Message::PenaltyShot(true)),
            )
            .style(ButtonStyle::Red)
        }
    };

    drop(tm);

    row![black, referee, penalty, white].spacing(SPACING)
}

pub(super) fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    tall: bool,
    editing_time: bool,
    mode: Mode,
    clock_running: bool,
) -> Row<'a, Message> {
    let make_red = if editing_time {
        false
    } else {
        match snapshot.timeout {
            TimeoutSnapshot::Black(time) | TimeoutSnapshot::White(time) => {
                (time <= 10 && (time % 2 == 0) && (time != 0)) || time == 15
            }
            TimeoutSnapshot::Ref(_) | TimeoutSnapshot::PenaltyShot(_) => false,
            TimeoutSnapshot::None => {
                let is_alert_period = match snapshot.current_period {
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
                        || (is_alert_period && snapshot.secs_in_period == 30))
            }
        }
    };

    let (mut period_text, period_color) = {
        let (text, color) = match snapshot.current_period {
            GamePeriod::BetweenGames => ("NEXT GAME", TextStyle::Yellow),
            GamePeriod::FirstHalf => ("FIRST HALF", TextStyle::Green),
            GamePeriod::HalfTime => ("HALF TIME", TextStyle::Yellow),
            GamePeriod::SecondHalf => ("SECOND HALF", TextStyle::Green),
            GamePeriod::PreOvertime => ("PRE OVERTIME BREAK", TextStyle::Yellow),
            GamePeriod::OvertimeFirstHalf => ("OVERTIME FIRST HALF", TextStyle::Green),
            GamePeriod::OvertimeHalfTime => ("OVERTIME HALF TIME", TextStyle::Yellow),
            GamePeriod::OvertimeSecondHalf => ("OVERTIME SECOND HALF", TextStyle::Green),
            GamePeriod::PreSuddenDeath => ("PRE SUDDEN DEATH BREAK", TextStyle::Yellow),
            GamePeriod::SuddenDeath => ("SUDDEN DEATH", TextStyle::Green),
        };

        if make_red {
            (text, TextStyle::Black)
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

    let make_time_view_row = |period_text, time_text, style| {
        let per = text(period_text)
            .line_height(LINE_HEIGHT)
            .style(style)
            .width(Length::Fill)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Right);
        let time = text(time_text)
            .line_height(LINE_HEIGHT)
            .style(style)
            .size(LARGE_TEXT)
            .width(Length::Fill)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Left);
        let r = row![].spacing(SPACING);
        make_time_view!(r, per, time)
    };

    let make_time_view_col = |period_text, time_text, style| {
        let per = text(period_text).line_height(LINE_HEIGHT).style(style);
        let time = text(time_text)
            .line_height(LINE_HEIGHT)
            .style(style)
            .size(LARGE_TEXT);
        let c = column![];
        make_time_view!(c, per, time)
    };

    let mut content = row![]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center);

    let timeout_info = match snapshot.timeout {
        TimeoutSnapshot::White(_) => Some((
            if tall { "WHT T/O" } else { "WHITE TIMEOUT" },
            if make_red {
                TextStyle::Black
            } else {
                TextStyle::White
            },
        )),
        TimeoutSnapshot::Black(_) => Some((
            if tall { "BLK T/O" } else { "BLACK TIMEOUT" },
            TextStyle::Black,
        )),
        TimeoutSnapshot::Ref(_) => Some(("REF TMOUT", TextStyle::Yellow)),
        TimeoutSnapshot::PenaltyShot(_) => Some(("PNLTY SHT", TextStyle::Red)),
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
        Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING)
    } else {
        Length::Fixed(MIN_BUTTON_SIZE)
    };

    let button_style = if make_red {
        ButtonStyle::Red
    } else {
        ButtonStyle::Gray
    };

    let time_button = button(content)
        .width(Length::Fill)
        .height(button_height)
        .style(button_style)
        .padding(PADDING)
        .on_press(if editing_time {
            Message::NoAction
        } else {
            Message::EditTime
        });

    let mut time_row = row![time_button]
        .height(button_height)
        .width(Length::Fill)
        .spacing(SPACING);

    if mode == Mode::Rugby {
        let play_pause_icon = container(
            Svg::new(svg::Handle::from_memory(if clock_running {
                &include_bytes!("../../../resources/pause.svg")[..]
            } else {
                &include_bytes!("../../../resources/play_arrow.svg")[..]
            }))
            .style(SvgStyle::Black)
            .height(Length::Fixed(LARGE_TEXT * 1.2)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(ContainerStyle::Transparent)
        .center_x()
        .center_y();
        let mut play_pause_button = button(play_pause_icon)
            .style(ButtonStyle::Gray)
            .height(button_height)
            .width(Length::Fixed(MIN_BUTTON_SIZE));
        if !editing_time {
            play_pause_button = play_pause_button.on_press(if clock_running {
                Message::StopClock
            } else {
                Message::StartClock
            });
        };
        time_row = time_row.push(play_pause_button);
    };

    time_row
}

pub(super) fn make_time_editor<'a, T: ToString>(
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'a, Message> {
    let wide = time > Duration::from_secs(MAX_STRINGABLE_SECS as u64);

    let min_edits = column![
        make_small_button("+", LARGE_TEXT)
            .style(ButtonStyle::Blue)
            .on_press(Message::ChangeTime {
                increase: true,
                secs: 60,
                timeout,
            }),
        make_small_button("-", LARGE_TEXT)
            .style(ButtonStyle::Blue)
            .on_press(Message::ChangeTime {
                increase: false,
                secs: 60,
                timeout,
            }),
    ]
    .spacing(SPACING);

    let sec_edits = column![
        make_small_button("+", LARGE_TEXT)
            .style(ButtonStyle::Blue)
            .on_press(Message::ChangeTime {
                increase: true,
                secs: 1,
                timeout,
            }),
        make_small_button("-", LARGE_TEXT)
            .style(ButtonStyle::Blue)
            .on_press(Message::ChangeTime {
                increase: false,
                secs: 1,
                timeout,
            }),
    ]
    .spacing(SPACING);

    let time_edit = row![
        min_edits,
        text(time_string(time))
            .size(LARGE_TEXT)
            .line_height(LINE_HEIGHT)
            .horizontal_alignment(Horizontal::Center)
            .width(Length::Fixed(if wide { 300.0 } else { 200.0 })),
        sec_edits,
    ]
    .spacing(SPACING)
    .align_items(Alignment::Center);

    container(
        column![
            text(title).size(MEDIUM_TEXT).line_height(LINE_HEIGHT),
            time_edit
        ]
        .spacing(SPACING)
        .align_items(Alignment::Center),
    )
    .style(ContainerStyle::LightGray)
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
    let black = limit_team_name_len(&game.black, len_limit);
    let white = limit_team_name_len(&game.white, len_limit);

    format!("{}{} - {} vs {}", game.game_type, game.gid, black, white)
}

pub(super) fn limit_team_name_len(name: &String, len_limit: usize) -> Cow<'_, String> {
    const ELIPSIS: [char; 3] = ['.', '.', '.'];

    if name.len() > len_limit {
        Cow::Owned(name.chars().take(len_limit - 1).chain(ELIPSIS).collect())
    } else {
        Cow::Borrowed(name)
    }
}

pub(super) fn config_string(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
    fouls_and_warnings: bool,
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
            "Last Game: {},  Next Game: {}\n\n",
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
        write!(&mut result, "Game: {}\n\n", game).unwrap();
        snapshot.game_number
    };

    if using_uwhscores {
        if let Some(games) = games {
            if let Some(game) = games.get(&game_number) {
                write!(
                    &mut result,
                    "Black Team: {}\nWhite Team: {}\n",
                    limit_team_name_len(&game.black, TEAM_NAME_LEN_LIMIT),
                    limit_team_name_len(&game.white, TEAM_NAME_LEN_LIMIT)
                )
                .unwrap()
            }
        }
    }

    writeln!(
        &mut result,
        "Half Length: {},  \
         Half Time Length: {}",
        time_string(config.half_play_duration),
        time_string(config.half_time_duration),
    )
    .unwrap();

    writeln!(
        &mut result,
        "Sudden Death Allowed: {},  \
         Overtime Allowed: {}",
        bool_string(config.sudden_death_allowed),
        bool_string(config.overtime_allowed),
    )
    .unwrap();

    writeln!(
        &mut result,
        "Team Timeouts Allowed Per Half: {}",
        config.team_timeouts_per_half
    )
    .unwrap();

    writeln!(&mut result, "Stop clock in last 2 minutes: ").unwrap();

    if !fouls_and_warnings {
        write!(
            &mut result,
            "Cheif ref: \n\
            Timer: \n\
            Water ref 1: \n\
            Water ref 2: \n\
            Water ref 3: ",
        )
        .unwrap();
    }

    result
}

pub(super) fn make_button<'a, Message: Clone, T: ToString>(label: T) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_multi_label_button<'a, Message: 'a + Clone, T: ToString>(
    labels: (T, T),
) -> Button<'a, Message> {
    button(
        column![centered_text(labels.0), centered_text(labels.1)]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding(PADDING)
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .width(Length::Fill)
}

pub fn centered_text<'a, T: ToString>(label: T) -> Text<'a> {
    text(label)
        .line_height(LINE_HEIGHT)
        .vertical_alignment(Vertical::Center)
        .horizontal_alignment(Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

pub(super) fn make_message_button<'a, Message: Clone, T: ToString>(
    label: T,
    message: Option<Message>,
) -> Button<'a, Message> {
    if let Some(msg) = message {
        make_button(label).on_press(msg)
    } else {
        make_button(label)
    }
}

pub(super) fn make_multi_label_message_button<'a, Message: 'a + Clone, T: ToString>(
    labels: (T, T),
    message: Option<Message>,
) -> Button<'a, Message> {
    if let Some(msg) = message {
        make_multi_label_button(labels).on_press(msg)
    } else {
        make_multi_label_button(labels)
    }
}

pub(super) fn make_small_button<'a, Message: Clone, T: ToString>(
    label: T,
    size: f32,
) -> Button<'a, Message> {
    button(centered_text(label).size(size))
        .width(Length::Fixed(MIN_BUTTON_SIZE))
        .height(Length::Fixed(MIN_BUTTON_SIZE))
}

pub(super) fn make_value_button<'a, Message: 'a + Clone, T: ToString, U: ToString>(
    first_label: T,
    second_label: U,
    large_text: (bool, bool),
    message: Option<Message>,
) -> Button<'a, Message> {
    let mut button = button(
        row![
            text(first_label)
                .size(if large_text.0 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center),
            horizontal_space(Length::Fill),
            text(second_label)
                .size(if large_text.1 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center),
        ]
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .padding(PADDING),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .style(ButtonStyle::LightGray);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

pub(super) fn make_penalty_dropdown<'a>(
    infraction: Infraction,
    expanded: bool,
) -> Element<'a, Message> {
    const ROW_LEN: usize = 6;

    let svg_file = if expanded {
        &include_bytes!("../../../resources/expand_more.svg")[..]
    } else {
        &include_bytes!("../../../resources/expand_less.svg")[..]
    };
    let closed_button_content = row![
        text("INFRACTION")
            .size(MEDIUM_TEXT)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Left)
            .height(Length::Fill)
            .line_height(LINE_HEIGHT),
        horizontal_space(Length::Fill),
        container(
            Svg::new(svg::Handle::from_memory(infraction.svg_fouls())).style(SvgStyle::Black)
        )
        .padding(PADDING)
        .style(ContainerStyle::LightGray)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(80),
        horizontal_space(Length::Fixed(SPACING)),
        container(
            Svg::new(svg::Handle::from_memory(svg_file,))
                .style(SvgStyle::White)
                .height(Length::Fixed(MEDIUM_TEXT * 1.3)),
        )
        .width(60)
        .height(Length::Fill)
        .style(ContainerStyle::Transparent)
        .center_y()
    ];

    let foul_dropdown = button(closed_button_content)
        .width(Length::Fill)
        .style(ButtonStyle::Blue);

    if expanded {
        let foul_buttons = all::<Infraction>().map(|button_infraction| {
            button(
                container(
                    Svg::new(svg::Handle::from_memory(button_infraction.svg_fouls()))
                        .style(SvgStyle::Black),
                )
                .style(ContainerStyle::LightGray),
            )
            .padding(0)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .width(Length::Fill)
            .style(if infraction == button_infraction {
                ButtonStyle::LightGraySelected
            } else {
                ButtonStyle::LightGray
            })
            .on_press(Message::ChangeInfraction(button_infraction))
        });

        let mut first_row = row![].spacing(SPACING);
        for button in foul_buttons.clone().take(ROW_LEN) {
            first_row = first_row.push(button);
        }
        let mut second_row = row![].spacing(SPACING);
        for button in foul_buttons.skip(ROW_LEN).take(ROW_LEN) {
            second_row = second_row.push(button);
        }

        let open_button_content = column![
            foul_dropdown
                .padding(0)
                .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                .on_press(Message::FoulSelectExpanded(false)),
            vertical_space(Length::Fixed(SPACING)),
            first_row,
            vertical_space(Length::Fixed(SPACING)),
            second_row,
        ]
        .padding(0);

        container(open_button_content)
            .padding(PADDING)
            .width(Length::Fill)
            .style(ContainerStyle::Blue)
            .into()
    } else {
        foul_dropdown
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .on_press(Message::FoulSelectExpanded(true))
            .into()
    }
}

pub fn make_warning_container<'a>(
    warning: &InfractionSnapshot,
    color: Option<GameColor>,
) -> Container<'a, Message> {
    const WIDTH: u16 = 220;
    const HEIGHT: u16 = 23;

    let who = if let Some(num) = warning.player_number {
        format!("#{num}")
    } else {
        "T".to_string()
    };

    container(if color.is_some() {
        row![
            horizontal_space(PADDING),
            text(warning.infraction.short_name()).size(SMALL_TEXT),
            horizontal_space(Length::Fill),
            text(who).size(SMALL_TEXT),
            horizontal_space(PADDING),
        ]
    } else {
        row![
            horizontal_space(Length::Fill),
            text(warning.infraction.short_name()).size(SMALL_TEXT),
            horizontal_space(Length::Fill),
        ]
    })
    .width(WIDTH)
    .height(HEIGHT)
    .style(match color {
        Some(GameColor::Black) => ContainerStyle::Black,
        Some(GameColor::White) => ContainerStyle::White,
        None => ContainerStyle::Blue,
    })
    .padding(0)
}
