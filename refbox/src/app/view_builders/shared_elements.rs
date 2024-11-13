use super::{
    Element, fl,
    message::*,
    style::{
        Button, ButtonStyle, Container, ContainerStyle, LARGE_TEXT, LINE_HEIGHT, MEDIUM_TEXT,
        MIN_BUTTON_SIZE, PADDING, Row, SMALL_PLUS_TEXT, SMALL_TEXT, SPACING, SvgStyle, Text,
        TextStyle, XS_BUTTON_SIZE,
    },
};
use crate::{config::Mode, tournament_manager::TournamentManager};
use enum_iterator::all;
use iced::{
    Alignment, Length,
    alignment::{Horizontal, Vertical},
    widget::{
        Image, button, column, container, horizontal_space, image, row,
        svg::{self, Svg},
        text, vertical_space,
    },
};
use matrix_drawing::{secs_to_long_time_string, secs_to_time_string};
use std::{
    fmt::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
    drawing_support::*,
    game_snapshot::{
        GamePeriod, GameSnapshot, Infraction, InfractionSnapshot, PenaltySnapshot, PenaltyTime,
        TimeoutSnapshot,
    },
    uwhportal::schedule::{Game, GameList, ResultOf, ScheduledTeam, TeamList},
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
        None => make_multi_label_message_button(
            (fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")),
            tm.can_start_team_timeout(GameColor::Black)
                .ok()
                .map(|_| Message::TeamTimeout(GameColor::Black, false)),
        )
        .style(ButtonStyle::Black),
        Some(TimeoutSnapshot::Black(_)) => make_multi_label_message_button(
            (fl!("end-timeout-line-1"), fl!("end-timeout-line-2")),
            Some(Message::EndTimeout),
        )
        .style(ButtonStyle::Yellow),
        Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => make_multi_label_message_button(
            (fl!("switch-to"), fl!("dark-team-name-caps")),
            tm.can_switch_to_team_timeout(GameColor::Black)
                .ok()
                .map(|_| Message::TeamTimeout(GameColor::Black, true)),
        )
        .style(ButtonStyle::Black),
    };

    let white = match snapshot.timeout {
        None => make_multi_label_message_button(
            (fl!("light-timeout-line-1"), fl!("light-timeout-line-2")),
            tm.can_start_team_timeout(GameColor::White)
                .ok()
                .map(|_| Message::TeamTimeout(GameColor::White, false)),
        )
        .style(ButtonStyle::White),
        Some(TimeoutSnapshot::White(_)) => make_multi_label_message_button(
            (fl!("end-timeout-line-1"), fl!("end-timeout-line-2")),
            Some(Message::EndTimeout),
        )
        .style(ButtonStyle::Yellow),
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => make_multi_label_message_button(
            (fl!("switch-to"), fl!("light-team-name-caps")),
            tm.can_switch_to_team_timeout(GameColor::White)
                .ok()
                .map(|_| Message::TeamTimeout(GameColor::White, true)),
        )
        .style(ButtonStyle::White),
    };

    let referee = match snapshot.timeout {
        None => make_multi_label_message_button(
            (fl!("ref-timeout-line-1"), fl!("ref-timeout-line-2")),
            tm.can_start_ref_timeout()
                .ok()
                .map(|_| Message::RefTimeout(false)),
        )
        .style(ButtonStyle::Yellow),
        Some(TimeoutSnapshot::Ref(_)) => make_multi_label_message_button(
            (fl!("end-timeout-line-1"), fl!("end-timeout-line-2")),
            Some(Message::EndTimeout),
        )
        .style(ButtonStyle::Yellow),
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => make_multi_label_message_button(
            (fl!("switch-to"), fl!("ref")),
            tm.can_switch_to_ref_timeout()
                .ok()
                .map(|_| Message::RefTimeout(true)),
        )
        .style(ButtonStyle::Yellow),
    };

    let penalty = match snapshot.timeout {
        None => make_multi_label_message_button(
            (fl!("penalty-shot-line-1"), fl!("penalty-shot-line-2")),
            tm.can_start_penalty_shot()
                .ok()
                .map(|_| Message::PenaltyShot(false)),
        )
        .style(ButtonStyle::Red),
        Some(TimeoutSnapshot::PenaltyShot(_)) => make_multi_label_message_button(
            (fl!("end-timeout-line-1"), fl!("end-timeout-line-2")),
            Some(Message::EndTimeout),
        )
        .style(ButtonStyle::Yellow),
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_)) => {
            let can_switch = if mode == Mode::Rugby {
                tm.can_switch_to_rugby_penalty_shot()
            } else {
                tm.can_switch_to_penalty_shot()
            };
            make_multi_label_message_button(
                (fl!("switch-to"), fl!("pen-shot")),
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
            Some(TimeoutSnapshot::Black(time)) | Some(TimeoutSnapshot::White(time)) => {
                (time <= 10 && (time % 2 == 0) && (time != 0)) || time == 15
            }
            Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => false,
            None => {
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
            GamePeriod::BetweenGames => (fl!("next-game"), TextStyle::Yellow),
            GamePeriod::FirstHalf => (fl!("first-half"), TextStyle::Green),
            GamePeriod::HalfTime => (fl!("half-time"), TextStyle::Yellow),
            GamePeriod::SecondHalf => (fl!("second-half"), TextStyle::Green),
            GamePeriod::PreOvertime => (fl!("pre-ot-break-full"), TextStyle::Yellow),
            GamePeriod::OvertimeFirstHalf => (fl!("overtime-first-half"), TextStyle::Green),
            GamePeriod::OvertimeHalfTime => (fl!("overtime-half-time"), TextStyle::Yellow),
            GamePeriod::OvertimeSecondHalf => (fl!("overtime-second-half"), TextStyle::Green),
            GamePeriod::PreSuddenDeath => (fl!("pre-sudden-death-break"), TextStyle::Yellow),
            GamePeriod::SuddenDeath => (fl!("sudden-death"), TextStyle::Green),
        };

        if make_red {
            (text, TextStyle::Black)
        } else {
            (text, color)
        }
    };

    if tall && (snapshot.timeout.is_some()) {
        match snapshot.current_period {
            GamePeriod::PreOvertime => period_text = fl!("pre-ot-break-abreviated"),
            GamePeriod::OvertimeFirstHalf => period_text = fl!("ot-first-half"),
            GamePeriod::OvertimeHalfTime => period_text = fl!("ot-half-time"),
            GamePeriod::OvertimeSecondHalf => period_text = fl!("ot-2nd-half"),
            GamePeriod::PreSuddenDeath => period_text = fl!("pre-sd-break"),
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

    let timeout_info = snapshot.timeout.map(|t| match t {
        TimeoutSnapshot::White(_) => (
            if tall {
                fl!("white-timeout-short")
            } else {
                fl!("white-timeout-full")
            },
            if make_red {
                TextStyle::Black
            } else {
                TextStyle::White
            },
        ),
        TimeoutSnapshot::Black(_) => (
            if tall {
                fl!("black-timeout-short")
            } else {
                fl!("black-timeout-full")
            },
            TextStyle::Black,
        ),
        TimeoutSnapshot::Ref(_) => (fl!("ref-timeout-short"), TextStyle::Yellow),
        TimeoutSnapshot::PenaltyShot(_) => (fl!("penalty-shot-short"), TextStyle::Red),
    });

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
        Some(TimeoutSnapshot::Black(secs))
        | Some(TimeoutSnapshot::White(secs))
        | Some(TimeoutSnapshot::Ref(secs))
        | Some(TimeoutSnapshot::PenaltyShot(secs)) => secs_to_time_string(secs).trim().to_string(),
        None => String::new(),
    }
}

pub(super) fn bool_string(val: bool) -> String {
    match val {
        true => fl!("yes"),
        false => fl!("no"),
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
                    string += &fl!("served");
                    string += "\n";
                }
            }
            PenaltyTime::TotalDismissal => {
                string += &fl!("dismissed");
                string += "\n"
            }
        }
    }
    // if the string is not empty, the last char is a '\n' that we don't want
    string.pop();
    string
}

pub(super) fn game_string_short(game: &Game) -> String {
    format!("{}{}", game.timing_rule, game.number)
}

pub(super) fn game_string_long(game: &Game, teams: Option<&TeamList>, len_limit: usize) -> String {
    let (black, white) = if let Some(teams) = teams {
        (
            get_team_name(&game.dark, teams),
            get_team_name(&game.light, teams),
        )
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
    };

    let black = limit_team_name_len(&black, len_limit);
    let white = limit_team_name_len(&white, len_limit);

    format!(
        "{}{} - {} vs {}",
        game.timing_rule, game.number, black, white
    )
}

pub(super) fn get_team_name(team: &ScheduledTeam, teams: &TeamList) -> String {
    if let Some(id) = team.assigned() {
        teams
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.full().to_string())
    } else if let Some(result_of) = team.result_of() {
        match result_of {
            ResultOf::Loser { game_number } => format!("L{game_number}"),
            ResultOf::Winner { game_number } => format!("W{game_number}"),
        }
    } else if let Some(seed) = team.seeded_by() {
        format!("Seed {} of {}", seed.number, seed.group)
    } else if let Some(s) = team.pending() {
        s.to_string()
    } else {
        "Unknown".to_string()
    }
}

pub(super) fn limit_team_name_len(name: &str, len_limit: usize) -> String {
    const ELIPSIS: [char; 3] = ['.', '.', '.'];

    if name.len() > len_limit {
        name.chars().take(len_limit - 1).chain(ELIPSIS).collect()
    } else {
        name.to_owned()
    }
}

pub(super) fn config_string_game_num(
    snapshot: &GameSnapshot,
    using_uwhportal: bool,
    games: Option<&GameList>,
) -> (String, u32) {
    let mut result = String::new();
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if using_uwhportal {
            if let Some(games) = games {
                prev_game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_short(game),
                    None if snapshot.game_number == 0 => fl!("none"),
                    None => fl!("error", number = snapshot.game_number),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game_string_short(game),
                    None => fl!("error", number = snapshot.next_game_number),
                };
            } else {
                prev_game = if snapshot.game_number == 0 {
                    fl!("none")
                } else {
                    fl!("error", number = snapshot.game_number)
                };
                next_game = fl!("error", number = snapshot.next_game_number);
            }
        } else {
            prev_game = if snapshot.game_number == 0 {
                fl!("none")
            } else {
                snapshot.game_number.to_string()
            };
            next_game = snapshot.next_game_number.to_string();
        }

        result += &fl!("two-games", prev_game = prev_game, next_game = next_game);
        result += "\n\n";
        snapshot.next_game_number
    } else {
        let game;
        if using_uwhportal {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game_string_short(game),
                    None => fl!("error", number = snapshot.game_number),
                };
            } else {
                game = fl!("error", number = snapshot.game_number);
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        result += &fl!("one-game", game = game);
        result += "\n\n";
        snapshot.game_number
    };

    (result, game_number)
}

pub(super) fn config_string(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    teams: Option<&TeamList>,
    fouls_and_warnings: bool,
) -> String {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let (result_string, _) = config_string_game_num(snapshot, using_uwhportal, games);
    let mut result = result_string;
    let (_, result_u32) = config_string_game_num(snapshot, using_uwhportal, games);
    let game_number = result_u32;

    if using_uwhportal {
        if let (Some(games), Some(teams)) = (games, teams) {
            if let Some(game) = games.get(&game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                result += &fl!(
                    "teams",
                    dark_team = limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    light_team = limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT)
                );
                result += "\n";
            }
        }
    }

    let unknown = &fl!("unknown");

    result += &fl!(
        "game-config",
        half_len = time_string(config.half_play_duration),
        half_time_len = time_string(config.half_time_duration),
        sd_allowed = bool_string(config.sudden_death_allowed),
        ot_allowed = bool_string(config.overtime_allowed)
    );

    if config.timeouts_counted_per_half {
        result += &fl!(
            "team-timeouts-per-half",
            team_timeouts = config.num_team_timeouts_allowed
        );
    } else {
        result += &fl!(
            "team-timeouts-per-game",
            team_timeouts = config.num_team_timeouts_allowed
        );
    }

    result += &fl!("stop-clock-last-2", stop_clock = unknown);
    result += "\n";

    if !fouls_and_warnings {
        result += &fl!(
            "ref-list",
            chief_ref = unknown,
            timer = unknown,
            water_ref_1 = unknown,
            water_ref_2 = unknown,
            water_ref_3 = unknown
        );
    }

    result
}

pub(super) fn make_button<'a, Message: Clone, T: ToString>(label: T) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_smaller_button<'a, Message: Clone, T: ToString>(
    label: T,
) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(XS_BUTTON_SIZE))
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
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .width(Length::Fill)
    .style(ButtonStyle::LightGray);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

pub(super) fn make_penalty_dropdown<'a>(
    infraction: Infraction,
    display_infraction_name: bool,
) -> Element<'a, Message> {
    const ROW_LEN: usize = 6;
    let foul_buttons = all::<Infraction>().map(|button_infraction| {
        button(
            container(
                Image::new(image::Handle::from_memory(button_infraction.get_image()))
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE)),
            )
            .style(ContainerStyle::Transparent),
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

    let name: Container<'_, Message> = container(
        row![text(format!("Infraction: {}", infraction))]
            .spacing(0)
            .align_items(Alignment::Center),
    )
    .style(ContainerStyle::Blue)
    .width(Length::Fill);

    let mut first_row = row![].spacing(SPACING);
    for button in foul_buttons.clone().take(ROW_LEN) {
        first_row = first_row.push(button);
    }
    let mut second_row = row![].spacing(SPACING);
    for button in foul_buttons.skip(ROW_LEN).take(ROW_LEN) {
        second_row = second_row.push(button);
    }

    let open_button_content = if display_infraction_name {
        column![
            name,
            vertical_space(Length::Fixed(SPACING)),
            first_row,
            vertical_space(Length::Fixed(SPACING)),
            second_row,
        ]
        .padding(0)
    } else {
        column![
            first_row,
            vertical_space(Length::Fixed(SPACING)),
            second_row,
        ]
        .padding(0)
    };

    container(open_button_content)
        .padding(PADDING)
        .width(Length::Fill)
        .style(ContainerStyle::Blue)
        .into()
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
        fl!("team-warning-abreviation")
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
