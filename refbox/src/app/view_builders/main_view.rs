use super::*;
use iced::{
    Alignment, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Space, button, column, container, mouse_area, row, text},
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot, PenaltyTime},
    uwhportal::schedule::Schedule,
};

// View builder takes app-state slices; grouping into a context struct is a separate refactor across all view_builders. Filed as a Findings-Backlog item in AUDIT-PLAN.md (Unit 3, 2026-05-13).
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn build_main_view<'a>(
    data: ViewData<'_, '_>,
    game_config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    track_fouls_and_warnings: bool,
    manual_alarm_enabled: bool,
    alarm_held: bool,
    overrun: std::time::Duration,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        portal_indicator,
        has_led_panel: _,
        ..
    } = data;

    let over_slot = overrun.saturating_sub(game_config.game_block_buffer());
    let overrun_label = if over_slot > std::time::Duration::ZERO {
        Some(format!("-{}", time_string(over_slot)))
    } else {
        None
    };
    let time_button = make_game_time_button(
        snapshot,
        true,
        false,
        mode,
        clock_running,
        portal_indicator,
        overrun_label,
    );

    let mut center_col = column![time_button]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill);

    let make_warn_button = || {
        make_button(fl!("add-warning"))
            .style(blue_button)
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
            .style(orange_button)
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
                    .style(yellow_button)
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
                        .style(green_button)
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

    if manual_alarm_enabled {
        center_col = center_col.push(
            make_button(fl!("game-info"))
                .style(light_gray_button)
                .on_press(Message::ShowGameDetails),
        );

        // Red + tap prompt during active play with no timeout; blue + hold prompt everywhere else.
        // The button is always interactive while the feature is enabled.
        let is_active_play = matches!(
            (snapshot.current_period, snapshot.timeout),
            (
                GamePeriod::FirstHalf
                    | GamePeriod::SecondHalf
                    | GamePeriod::OvertimeFirstHalf
                    | GamePeriod::OvertimeSecondHalf
                    | GamePeriod::SuddenDeath,
                None,
            )
        );
        let alarm_label = if is_active_play {
            fl!("alarm")
        } else {
            fl!("hold-to-test")
        };
        let spacebar_label = if is_active_play {
            fl!("or-press-spacebar")
        } else {
            fl!("or-hold-spacebar")
        };
        let alarm_face_container = container(
            column![
                text(alarm_label)
                    .size(SMALL_PLUS_TEXT)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
                text(spacebar_label)
                    .size(SMALL_TEXT)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill),
        )
        .style(match (is_active_play, alarm_held) {
            (true, true) => red_pressed_container,
            (true, false) => red_container,
            (false, true) => blue_pressed_container,
            (false, false) => blue_container,
        })
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

        let alarm_face: Element<'a, Message> = mouse_area(alarm_face_container)
            .on_press(Message::AlarmPressed)
            .on_release(Message::AlarmReleased)
            .into();

        if track_fouls_and_warnings {
            // Split the lower area: alarm left, warnings right
            let warnings_zone = button(
                column![
                    text(fl!("warnings"))
                        .align_y(Vertical::Top)
                        .align_x(Horizontal::Center)
                        .width(Length::Fill),
                    row(snapshot.warnings.iter().map(|(color, warns)| column(
                        warns
                            .iter()
                            .rev()
                            .take(10)
                            .map(|warning| make_warning_container(warning, Some(color)).into())
                    )
                    .spacing(1)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()))
                    .spacing(SPACING),
                ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(light_gray_button)
            .on_press(Message::ShowWarnings);

            center_col = center_col.push(
                row![alarm_face, warnings_zone]
                    .spacing(SPACING)
                    .height(Length::Fill),
            );
        } else {
            center_col = center_col.push(alarm_face);
        }
    } else {
        // Original behavior: game info area + optional warnings panel
        // Centering via container wrapper rather than text.align_y avoids
        // the iced 0.13 paragraph-cache stale-position bug; same fix
        // applied in commit 8a8d018 for make_multi_label_button.
        center_col = center_col.push(if max_num_warns < 4 {
            button(
                container(
                    text(config_string(
                        snapshot,
                        game_config,
                        using_uwhportal,
                        schedule,
                        teams,
                    ))
                    .size(SMALL_TEXT)
                    .align_x(Horizontal::Left)
                    .width(Length::Fill),
                )
                .center_y(Length::Fill)
                .width(Length::Fill),
            )
            .padding(PADDING)
            .style(light_gray_button)
            .height(Length::FillPortion(2))
            .width(Length::Fill)
            .on_press(Message::ShowGameDetails)
        } else {
            button(
                container(
                    text(
                        config_string_game_num(
                            snapshot,
                            using_uwhportal,
                            schedule.map(|s| &s.games),
                        )
                        .0,
                    )
                    .size(SMALL_TEXT)
                    .align_x(Horizontal::Left)
                    .width(Length::Fill),
                )
                .center_y(Length::Fill)
                .width(Length::Fill),
            )
            .padding(PADDING)
            .style(light_gray_button)
            .width(Length::Fill)
            .on_press(Message::ShowGameDetails)
        });

        if track_fouls_and_warnings {
            center_col = center_col.push(
                button(
                    column![
                        text(fl!("warnings"))
                            .align_y(Vertical::Top)
                            .align_x(Horizontal::Center)
                            .width(Length::Fill),
                        row(snapshot.warnings.iter().map(|(color, warns)| column(
                            warns
                                .iter()
                                .rev()
                                .take(10)
                                .map(|warning| make_warning_container(warning, Some(color)).into())
                        )
                        .spacing(1)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()))
                        .spacing(SPACING),
                    ]
                    .spacing(0)
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(light_gray_button)
                .on_press(Message::ShowWarnings),
            )
        }
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
            red_button
        } else {
            match color {
                GameColor::Black => black_button,
                GameColor::White => white_button,
            }
        };

        button(
            column![
                text(fl!("penalties"))
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
                text(penalty_string(penalties))
                    .align_y(Vertical::Top)
                    .align_x(Horizontal::Left)
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
            text(fl!("dark-team-name-caps")),
            text(snapshot.scores.black.to_string()).size(LARGE_TEXT),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(black_button);

    let mut black_new_score_btn =
        make_multi_label_button((fl!("dark-score-line-1"), fl!("dark-score-line-2")))
            .style(black_button);

    let mut white_score_btn = button(
        column![
            text(fl!("light-team-name-caps")),
            text(snapshot.scores.white.to_string()).size(LARGE_TEXT),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Fixed(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(white_button);

    let mut white_new_score_btn =
        make_multi_label_button((fl!("light-score-line-1"), fl!("light-score-line-2")))
            .style(white_button);

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
    .align_x(Alignment::Center)
    .width(Length::Fill);

    let white_col = column![
        white_score_btn,
        white_new_score_btn,
        make_penalty_button(snapshot, GameColor::White),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill);

    row![
        row![
            black_col,
            Space::with_width(Length::Fixed(3.0 * SPACING / 4.0)),
        ]
        .width(Length::Fill)
        .spacing(0),
        row![
            Space::with_width(Length::Fixed(SPACING / 4.0)),
            center_col,
            Space::with_width(Length::Fixed(SPACING / 4.0)),
        ]
        .width(Length::FillPortion(2))
        .spacing(0),
        row![
            Space::with_width(Length::Fixed(3.0 * SPACING / 4.0)),
            white_col,
        ]
        .width(Length::Fill)
        .spacing(0),
    ]
    .spacing(0)
    .height(Length::Fill)
    .into()
}
