use super::{
    style::{
        ButtonStyle, Element, LARGE_TEXT, LINE_HEIGHT, MIN_BUTTON_SIZE, PADDING, SMALL_PLUS_TEXT,
        SMALL_TEXT, SPACING,
    },
    *,
};
use iced::{
    Alignment, Length,
    alignment::{Horizontal, Vertical},
    widget::{button, column, horizontal_space, row, text},
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot, PenaltyTime},
    uwhportal::schedule::GameList,
};

pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    games: Option<&GameList>,
    track_fouls_and_warnings: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
    } = data;

    let time_button = make_game_time_button(snapshot, true, false, mode, clock_running);

    let mut center_col = column![time_button].spacing(SPACING).width(Length::Fill);

    let make_warn_button = || {
        make_button(fl!("add-warning"))
            .style(ButtonStyle::Blue)
            .width(Length::Fill)
            .on_press(Message::KeypadPage(KeypadPage::WarningAdd {
                origin: None,
                color: GameColor::Black,
                infraction: Infraction::Unknown,
                team_warning: false,
                ret_to_overview: false,
            }))
    };

    let make_foul_button = || {
        make_button(fl!("add-foul"))
            .style(ButtonStyle::Orange)
            .width(Length::Fill)
            .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                origin: None,
                color: None,
                infraction: Infraction::Unknown,
                ret_to_overview: false,
            }))
    };

    if snapshot.timeout.is_some() {
        if track_fouls_and_warnings {
            center_col =
                center_col.push(row![make_foul_button(), make_warn_button()].spacing(SPACING));
        } else {
            center_col = center_col.push(
                make_button(fl!("end-timeout"))
                    .style(ButtonStyle::Yellow)
                    .on_press(Message::EndTimeout),
            );
        }
    } else {
        match snapshot.current_period {
            GamePeriod::BetweenGames
            | GamePeriod::HalfTime
            | GamePeriod::PreOvertime
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::PreSuddenDeath => {
                let mut start_warning_row = row![
                    make_button(fl!("start-now"))
                        .style(ButtonStyle::Green)
                        .width(Length::Fill)
                        .on_press(Message::StartPlayNow)
                ]
                .spacing(SPACING);

                if track_fouls_and_warnings {
                    start_warning_row = start_warning_row.push(make_warn_button())
                }

                center_col = center_col.push(start_warning_row)
            }
            GamePeriod::FirstHalf
            | GamePeriod::SecondHalf
            | GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::SuddenDeath => {
                if track_fouls_and_warnings {
                    center_col = center_col
                        .push(row![make_foul_button(), make_warn_button()].spacing(SPACING))
                }
            }
        };
    }

    let max_num_warns = snapshot
        .warnings
        .iter()
        .map(|(_, w)| w.len())
        .max()
        .unwrap();

    center_col = center_col.push(if max_num_warns < 4 {
        button(
            text(config_string(
                snapshot,
                game_config,
                using_uwhportal,
                games,
                teams,
                track_fouls_and_warnings,
            ))
            .size(SMALL_TEXT)
            .line_height(LINE_HEIGHT)
            .vertical_alignment(Vertical::Center)
            .horizontal_alignment(Horizontal::Left),
        )
        .padding(PADDING)
        .style(ButtonStyle::LightGray)
        .height(Length::FillPortion(2))
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails)
    } else {
        button(
            text(config_string_game_num(snapshot, using_uwhportal, games).0)
                .size(SMALL_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Left),
        )
        .padding(PADDING)
        .style(ButtonStyle::LightGray)
        .width(Length::Fill)
        .on_press(Message::ShowGameDetails)
    });

    if track_fouls_and_warnings {
        center_col = center_col.push(
            button(
                column![
                    text(fl!("warnings"))
                        .line_height(LINE_HEIGHT)
                        .vertical_alignment(Vertical::Top)
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill),
                    row(snapshot
                        .warnings
                        .iter()
                        .map(|(color, warns)| column(
                            warns
                                .iter()
                                .rev()
                                .take(10)
                                .map(|warning| make_warning_container(warning, Some(color)).into())
                                .collect()
                        )
                        .spacing(1)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into())
                        .collect())
                    .spacing(SPACING),
                ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(Message::NoAction)
            .style(ButtonStyle::LightGray)
            .on_press(Message::ShowWarnings),
        )
    }

    let make_penalty_button = |snapshot: &GameSnapshot, color: GameColor| {
        let penalties = &snapshot.penalties[color];

        let time = penalties
            .iter()
            .filter_map(|penalty| match penalty.time {
                PenaltyTime::Seconds(s) if s != 0 => Some(s),
                PenaltyTime::Seconds(_) => None,
                PenaltyTime::TotalDismissal => None,
            })
            .min();

        let make_penalties_red = if snapshot.timeout.is_none() {
            if let Some(t) = time {
                t <= 10 && (t % 2 == 0) && (t != 0)
            } else {
                false
            }
        } else {
            false
        };

        let button_style = if make_penalties_red {
            ButtonStyle::Red
        } else {
            match color {
                GameColor::Black => ButtonStyle::Black,
                GameColor::White => ButtonStyle::White,
            }
        };

        button(
            column![
                text(fl!("penalties"))
                    .line_height(LINE_HEIGHT)
                    .vertical_alignment(Vertical::Center)
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill),
                text(penalty_string(penalties))
                    .line_height(LINE_HEIGHT)
                    .vertical_alignment(Vertical::Top)
                    .horizontal_alignment(Horizontal::Left)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::PenaltyOverview)
        .style(button_style)
    };

    let mut black_score_btn = button(
        column![
            text(fl!("dark-team-name-caps")).line_height(LINE_HEIGHT),
            text(snapshot.scores.black.to_string())
                .size(LARGE_TEXT)
                .line_height(LINE_HEIGHT),
        ]
        .align_items(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(ButtonStyle::Black);

    let mut black_new_score_btn =
        make_multi_label_button((fl!("dark-score-line-1"), fl!("dark-score-line-2")))
            .style(ButtonStyle::Black);

    let mut white_score_btn = button(
        column![
            text(fl!("light-team-name-caps")).line_height(LINE_HEIGHT),
            text(snapshot.scores.white.to_string())
                .size(LARGE_TEXT)
                .line_height(LINE_HEIGHT),
        ]
        .align_items(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(ButtonStyle::White);

    let mut white_new_score_btn =
        make_multi_label_button((fl!("light-score-line-1"), fl!("light-score-line-2")))
            .style(ButtonStyle::White);

    if snapshot.current_period != GamePeriod::BetweenGames {
        black_score_btn = black_score_btn.on_press(Message::EditScores);
        black_new_score_btn = black_new_score_btn.on_press(Message::AddNewScore(GameColor::Black));
        white_score_btn = white_score_btn.on_press(Message::EditScores);
        white_new_score_btn = white_new_score_btn.on_press(Message::AddNewScore(GameColor::White));
    }

    let black_col = column![
        black_score_btn,
        black_new_score_btn,
        make_penalty_button(snapshot, GameColor::Black),
    ]
    .spacing(SPACING)
    .align_items(Alignment::Center)
    .width(Length::Fill);

    let white_col = column![
        white_score_btn,
        white_new_score_btn,
        make_penalty_button(snapshot, GameColor::White),
    ]
    .spacing(SPACING)
    .align_items(Alignment::Center)
    .width(Length::Fill);

    row![
        row![
            black_col,
            horizontal_space(Length::Fixed(3.0 * SPACING / 4.0)),
        ]
        .width(Length::Fill)
        .spacing(0),
        row![
            horizontal_space(Length::Fixed(SPACING / 4.0)),
            center_col,
            horizontal_space(Length::Fixed(SPACING / 4.0)),
        ]
        .width(Length::FillPortion(2))
        .spacing(0),
        row![
            horizontal_space(Length::Fixed(3.0 * SPACING / 4.0)),
            white_col,
        ]
        .width(Length::Fill)
        .spacing(0),
    ]
    .spacing(0)
    .height(Length::Fill)
    .into()
}
