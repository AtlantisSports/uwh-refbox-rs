use super::*;
use enum_iterator::all;
use iced::{
    Alignment, Background, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{
        Button, Container, Image, Row, Space, Text, button, column, container,
        container::Style as ContainerStyle, horizontal_space, image, row, svg, svg::Svg, text,
        text::Style as TextStyle, vertical_space,
    },
};
use iced_core::text::IntoFragment;
use matrix_drawing::{secs_to_long_time_string, secs_to_time_string};
use std::{
    fmt::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
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
    cont_style: fn(&Theme) -> ContainerStyle,
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

    let scroll_btn_style =
        if cont_style(&Theme::default()).background == Some(Background::Color(BLUE)) {
            blue_with_border_button
        } else {
            blue_button
        };

    let mut up_btn = button(
        container(
            Svg::new(svg::Handle::from_memory(
                &include_bytes!("../../../resources/arrow_drop_up.svg")[..],
            ))
            .style(white_svg),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_container),
    )
    .width(Length::Fixed(MIN_BUTTON_SIZE))
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(scroll_btn_style);

    let mut down_btn = button(
        container(
            Svg::new(svg::Handle::from_memory(
                &include_bytes!("../../../resources/arrow_drop_down.svg")[..],
            ))
            .style(white_svg),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_container),
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
        .push(horizontal_space())
        .push(
            container(column![
                Space::with_height(top_len),
                container(vertical_space())
                    .width(Length::Fill)
                    .height(Length::FillPortion(LIST_LEN as u16))
                    .style(gray_container),
                Space::with_height(bottom_len),
            ])
            .padding(PADDING)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(scroll_bar_container),
        )
        .push(horizontal_space());

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
        None => make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_team_timeout(GameColor::Black)
                    .ok()
                    .map(|_| Message::TeamTimeout(GameColor::Black, false)),
            )
            .style(black_button),
        Some(TimeoutSnapshot::Black(_)) => {
            make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(yellow_button)
        }
        Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("switch-to"), fl!("dark-team-name-caps")))
                .on_press_maybe(
                    tm.can_switch_to_team_timeout(GameColor::Black)
                        .ok()
                        .map(|_| Message::TeamTimeout(GameColor::Black, true)),
                )
                .style(black_button)
        }
    };

    let white = match snapshot.timeout {
        None => make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_team_timeout(GameColor::White)
                    .ok()
                    .map(|_| Message::TeamTimeout(GameColor::White, false)),
            )
            .style(white_button),
        Some(TimeoutSnapshot::White(_)) => {
            make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(yellow_button)
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("switch-to"), fl!("light-team-name-caps")))
                .on_press_maybe(
                    tm.can_switch_to_team_timeout(GameColor::White)
                        .ok()
                        .map(|_| Message::TeamTimeout(GameColor::White, true)),
                )
                .style(white_button)
        }
    };

    let referee = match snapshot.timeout {
        None => make_multi_label_button((fl!("ref-timeout-line-1"), fl!("ref-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_ref_timeout()
                    .ok()
                    .map(|_| Message::RefTimeout(false)),
            )
            .style(yellow_button),
        Some(TimeoutSnapshot::Ref(_)) => {
            make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(yellow_button)
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("switch-to"), fl!("ref")))
                .on_press_maybe(
                    tm.can_switch_to_ref_timeout()
                        .ok()
                        .map(|_| Message::RefTimeout(true)),
                )
                .style(yellow_button)
        }
    };

    let penalty = match snapshot.timeout {
        None => make_multi_label_button((fl!("penalty-shot-line-1"), fl!("penalty-shot-line-2")))
            .on_press_maybe(
                tm.can_start_penalty_shot()
                    .ok()
                    .map(|_| Message::PenaltyShot(false)),
            )
            .style(red_button),
        Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                .on_press(Message::EndTimeout)
                .style(yellow_button)
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_)) => {
            let can_switch = if mode == Mode::Rugby {
                tm.can_switch_to_rugby_penalty_shot()
            } else {
                tm.can_switch_to_penalty_shot()
            };
            make_multi_label_button((fl!("switch-to"), fl!("pen-shot")))
                .on_press_maybe(can_switch.ok().map(|_| Message::PenaltyShot(true)))
                .style(red_button)
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

    let (mut period_text, period_color): (_, fn(&Theme) -> TextStyle) = {
        let (text, color): (_, fn(&Theme) -> TextStyle) = match snapshot.current_period {
            GamePeriod::BetweenGames => (fl!("next-game"), yellow_text),
            GamePeriod::FirstHalf => (fl!("first-half"), green_text),
            GamePeriod::HalfTime => (fl!("half-time"), yellow_text),
            GamePeriod::SecondHalf => (fl!("second-half"), green_text),
            GamePeriod::PreOvertime => (fl!("pre-ot-break-full"), yellow_text),
            GamePeriod::OvertimeFirstHalf => (fl!("overtime-first-half"), green_text),
            GamePeriod::OvertimeHalfTime => (fl!("overtime-half-time"), yellow_text),
            GamePeriod::OvertimeSecondHalf => (fl!("overtime-second-half"), green_text),
            GamePeriod::PreSuddenDeath => (fl!("pre-sudden-death-break"), yellow_text),
            GamePeriod::SuddenDeath => (fl!("sudden-death"), green_text),
        };

        if make_red {
            (text, black_text)
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
            $base.width(Length::Fill).push($per_text).push($time_text)
        };
    }

    let make_time_view_row = |period_text, time_text, style: fn(&Theme) -> TextStyle| {
        let per = text(period_text)
            .style(style)
            .width(Length::Fill)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Right);
        let time = text(time_text)
            .style(style)
            .size(LARGE_TEXT)
            .width(Length::Fill)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Left);
        let r = row![].spacing(SPACING);
        make_time_view!(r, per, time).align_y(Alignment::Center)
    };

    let make_time_view_col = |period_text, time_text, style| {
        let per = text(period_text).style(style);
        let time = text(time_text).style(style).size(LARGE_TEXT);
        let c = column![];
        make_time_view!(c, per, time).align_x(Alignment::Center)
    };

    let mut content = row![]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_y(Alignment::Center);

    let timeout_info = snapshot.timeout.map(|t| -> (_, fn(&Theme) -> TextStyle) {
        match t {
            TimeoutSnapshot::White(_) => (
                if tall {
                    fl!("white-timeout-short")
                } else {
                    fl!("white-timeout-full")
                },
                if make_red { black_text } else { white_text },
            ),
            TimeoutSnapshot::Black(_) => (
                if tall {
                    fl!("black-timeout-short")
                } else {
                    fl!("black-timeout-full")
                },
                black_text,
            ),
            TimeoutSnapshot::Ref(_) => (fl!("ref-timeout-short"), yellow_text),
            TimeoutSnapshot::PenaltyShot(_) => (fl!("penalty-shot-short"), red_text),
        }
    });

    let time_text = secs_to_long_time_string(snapshot.secs_in_period);

    let time_text = time_text.trim().to_owned();

    if tall {
        content = content.push(make_time_view_col(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_col(
                timeout_text,
                timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    } else {
        content = content.push(make_time_view_row(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_row(
                timeout_text,
                timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    }

    let button_height = if tall {
        Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING)
    } else {
        Length::Fixed(MIN_BUTTON_SIZE)
    };

    let button_style = if make_red { red_button } else { gray_button };

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
            .style(black_svg)
            .height(Length::Fixed(LARGE_TEXT * 1.2)),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_container);
        let mut play_pause_button = button(play_pause_icon)
            .style(gray_button)
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

pub(super) fn make_time_editor<'a, T: IntoFragment<'a>>(
    title: T,
    time: Duration,
    timeout: bool,
) -> Container<'a, Message> {
    let wide = time > Duration::from_secs(MAX_STRINGABLE_SECS as u64);

    let min_edits = column![
        make_small_button("+", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: true,
                secs: 60,
                timeout,
            }),
        make_small_button("-", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: false,
                secs: 60,
                timeout,
            }),
    ]
    .spacing(SPACING);

    let sec_edits = column![
        make_small_button("+", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: true,
                secs: 1,
                timeout,
            }),
        make_small_button("-", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: false,
                secs: 1,
                timeout,
            }),
    ]
    .spacing(SPACING);

    let mut time_col = column![text(time_string(time)).size(LARGE_TEXT),]
        .align_x(Horizontal::Center)
        .width(Length::Fixed(if wide { 300.0 } else { 200.0 }))
        .spacing(SPACING);

    if wide {
        time_col = time_col.push(row![
            horizontal_space(),
            make_smaller_button(fl!("zero"))
                .style(blue_button)
                .on_press(Message::ChangeTime {
                    increase: false,
                    secs: u64::MAX,
                    timeout,
                }),
            horizontal_space(),
        ]);
    }

    let time_edit = row![min_edits, time_col, sec_edits]
        .spacing(SPACING)
        .align_y(Alignment::Center);

    container(
        column![text(title).size(MEDIUM_TEXT), time_edit]
            .spacing(SPACING)
            .align_x(Alignment::Center),
    )
    .style(light_gray_container)
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
                string += &fl!("total-dismissal");
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
        result += "\n";
        result += &fl!(
            "team-timeouts-per-half",
            team_timeouts = config.num_team_timeouts_allowed
        );
    } else {
        result += "\n";
        result += &fl!(
            "team-timeouts-per-game",
            team_timeouts = config.num_team_timeouts_allowed
        );
    }

    result += "\n";
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

pub(super) fn make_button<'a, Message: Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_smaller_button<'a, Message: Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    button(centered_text(label))
        .padding(PADDING)
        .height(Length::Fixed(XS_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_multi_label_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
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

pub fn centered_text<'a, T: IntoFragment<'a>>(label: T) -> Text<'a> {
    text(label)
        .align_y(Vertical::Center)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

pub(super) fn make_small_button<'a, Message: Clone, T: IntoFragment<'a>>(
    label: T,
    size: f32,
) -> Button<'a, Message> {
    button(centered_text(label).size(size))
        .width(Length::Fixed(MIN_BUTTON_SIZE))
        .height(Length::Fixed(MIN_BUTTON_SIZE))
}

pub(super) fn make_value_button<'a, T, U>(
    first_label: T,
    second_label: U,
    large_text: (bool, bool),
    message: Option<Message>,
) -> Button<'a, Message>
where
    Message: 'a + Clone,
    T: IntoFragment<'a>,
    U: IntoFragment<'a>,
{
    let mut button = button(
        row![
            text(first_label)
                .size(if large_text.0 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .height(Length::Fill)
                .align_y(Vertical::Center),
            horizontal_space(),
            text(second_label)
                .size(if large_text.1 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .height(Length::Fill)
                .align_y(Vertical::Center),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center)
        .padding(PADDING),
    )
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .width(Length::Fill)
    .style(light_gray_button);

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
                Image::new(image::Handle::from_bytes(button_infraction.get_image()))
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE)),
            )
            .style(transparent_container),
        )
        .padding(0)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .style(if infraction == button_infraction {
            light_gray_selected_button
        } else {
            light_gray_button
        })
        .on_press(Message::ChangeInfraction(button_infraction))
    });

    let name: Container<'_, Message> = container(
        row![text(fl!(
            "infraction",
            infraction = inf_short_name(infraction)
        ))]
        .spacing(0)
        .align_y(Alignment::Center),
    )
    .style(blue_container)
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
            Space::with_height(SPACING),
            first_row,
            Space::with_height(SPACING),
            second_row,
        ]
        .padding(0)
    } else {
        column![first_row, Space::with_height(SPACING), second_row,].padding(0)
    };

    container(open_button_content)
        .padding(PADDING)
        .width(Length::Fill)
        .style(blue_container)
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
            Space::with_width(PADDING),
            text(inf_short_name(warning.infraction)).size(SMALL_TEXT),
            horizontal_space(),
            text(who).size(SMALL_TEXT),
            Space::with_width(PADDING),
        ]
    } else {
        row![
            horizontal_space(),
            text(inf_short_name(warning.infraction)).size(SMALL_TEXT),
            horizontal_space(),
        ]
    })
    .width(WIDTH)
    .height(HEIGHT)
    .style(match color {
        Some(GameColor::Black) => black_container,
        Some(GameColor::White) => white_container,
        None => blue_container,
    })
    .padding(0)
}

pub fn inf_short_name(inf: Infraction) -> String {
    match inf {
        Infraction::Unknown => fl!("unknown"),
        Infraction::StickInfringement => fl!("stick-foul"),
        Infraction::IllegalAdvancement => fl!("illegal-advance"),
        Infraction::IllegalSubstitution => fl!("sub-foul"),
        Infraction::IllegallyStoppingThePuck => fl!("illegal-stoppage"),
        Infraction::OutOfBounds => fl!("out-of-bounds"),
        Infraction::GrabbingTheBarrier => fl!("grabbing-the-wall"),
        Infraction::Obstruction => fl!("obstruction"),
        Infraction::DelayOfGame => fl!("delay-of-game"),
        Infraction::UnsportsmanlikeConduct => fl!("unsportsmanlike"),
        Infraction::FreeArm => fl!("free-arm"),
        Infraction::FalseStart => fl!("false-start"),
    }
}
