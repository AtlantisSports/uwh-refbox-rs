use super::{
    super::Config,
    style::{
        ButtonStyle, Element, LARGE_TEXT, LINE_HEIGHT, MIN_BUTTON_SIZE, PADDING, SMALL_PLUS_TEXT,
        SMALL_TEXT, SPACING,
    },
    *,
};

use iced::{
    alignment::{Horizontal, Vertical},
    widget::{button, column, horizontal_space, row, text},
    Alignment, Length,
};
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{Color as GameColor, GamePeriod, GameSnapshot, PenaltyTime, TimeoutSnapshot},
};

pub(in super::super) fn build_main_view<'a>(
    snapshot: &GameSnapshot,
    game_config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
    config: &Config,
    clock_running: bool,
) -> Element<'a, Message> {
    let time_button = make_game_time_button(snapshot, true, false, config.mode, clock_running);

    let mut center_col = column![time_button].spacing(SPACING).width(Length::Fill);

    let make_warn_button = || {
        make_button("ADD WARNING")
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
        make_button("ADD FOUL")
            .style(ButtonStyle::Orange)
            .width(Length::Fill)
            .on_press(Message::KeypadPage(KeypadPage::FoulAdd {
                origin: None,
                color: None,
                infraction: Infraction::Unknown,
                ret_to_overview: false,
            }))
    };

    match snapshot.timeout {
        TimeoutSnapshot::White(_)
        | TimeoutSnapshot::Black(_)
        | TimeoutSnapshot::Ref(_)
        | TimeoutSnapshot::PenaltyShot(_) => {
            if config.track_fouls_and_warnings {
                center_col =
                    center_col.push(row![make_foul_button(), make_warn_button()].spacing(SPACING))
            } else {
                center_col = center_col.push(
                    make_button("END TIMEOUT")
                        .style(ButtonStyle::Yellow)
                        .on_press(Message::EndTimeout),
                )
            }
        }
        TimeoutSnapshot::None => {
            match snapshot.current_period {
                GamePeriod::BetweenGames
                | GamePeriod::HalfTime
                | GamePeriod::PreOvertime
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::PreSuddenDeath => {
                    let mut start_warning_row = row![make_button("START NOW")
                        .style(ButtonStyle::Green)
                        .width(Length::Fill)
                        .on_press(Message::StartPlayNow)]
                    .spacing(SPACING);

                    if config.track_fouls_and_warnings {
                        start_warning_row = start_warning_row.push(make_warn_button())
                    }

                    center_col = center_col.push(start_warning_row)
                }
                GamePeriod::FirstHalf
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::SuddenDeath => {
                    if config.track_fouls_and_warnings {
                        center_col = center_col
                            .push(row![make_foul_button(), make_warn_button()].spacing(SPACING))
                    }
                }
            };
        }
    };

    let num_warns_b = snapshot.b_warnings.len();
    let num_warns_w = snapshot.w_warnings.len();

    center_col = center_col.push(if num_warns_b | num_warns_w < 4 {
        button(
            text(config_string(
                snapshot,
                game_config,
                using_uwhscores,
                games,
                config.track_fouls_and_warnings,
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
            text(config_string_game_num(snapshot, using_uwhscores, games).0)
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

    if config.track_fouls_and_warnings {
        center_col = center_col.push(
            button(
                column![
                    text("WARNINGS")
                        .line_height(LINE_HEIGHT)
                        .vertical_alignment(Vertical::Top)
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill),
                    row![
                        column(
                            snapshot
                                .b_warnings
                                .iter()
                                .rev()
                                .take(10)
                                .map(|warning| make_warning_container(
                                    warning,
                                    Some(GameColor::Black)
                                )
                                .into())
                                .collect()
                        )
                        .spacing(1)
                        .width(Length::Fill)
                        .height(Length::Fill),
                        column(
                            snapshot
                                .w_warnings
                                .iter()
                                .rev()
                                .take(10)
                                .map(|warning| make_warning_container(
                                    warning,
                                    Some(GameColor::White)
                                )
                                .into())
                                .collect()
                        )
                        .spacing(1)
                        .width(Length::Fill)
                        .height(Length::Fill),
                    ]
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
        let penalties = match color {
            GameColor::Black => &snapshot.b_penalties,
            GameColor::White => &snapshot.w_penalties,
        };

        let time = penalties
            .iter()
            .filter_map(|penalty| match penalty.time {
                PenaltyTime::Seconds(s) if s != 0 => Some(s),
                PenaltyTime::Seconds(_) => None,
                PenaltyTime::TotalDismissal => None,
            })
            .min();

        let make_penalties_red = if snapshot.timeout == TimeoutSnapshot::None {
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
                text("PENALTIES")
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
            text("BLACK").line_height(LINE_HEIGHT),
            text(snapshot.b_score.to_string())
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
        make_multi_label_button(("SCORE", "BLACK")).style(ButtonStyle::Black);

    let mut white_score_btn = button(
        column![
            text("WHITE").line_height(LINE_HEIGHT),
            text(snapshot.w_score.to_string())
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
        make_multi_label_button(("SCORE", "WHITE")).style(ButtonStyle::White);

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
