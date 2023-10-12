use super::{
    style::{self, LARGE_TEXT, MIN_BUTTON_SIZE, PADDING, SMALL_PLUS_TEXT, SMALL_TEXT, SPACING},
    *,
};

use iced::{
    alignment::{Horizontal, Vertical},
    pure::{button, column, horizontal_space, row, text, Element},
    Alignment, Length,
};

use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{Color as GameColor, GamePeriod, GameSnapshot, PenaltyTime, TimeoutSnapshot},
};

pub(in super::super) fn build_main_view<'a>(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let time_button = make_game_time_button(snapshot, true, false, mode, clock_running);

    let mut center_col = column()
        .spacing(SPACING)
        .width(Length::Fill)
        .push(time_button);

    match snapshot.timeout {
        TimeoutSnapshot::White(_)
        | TimeoutSnapshot::Black(_)
        | TimeoutSnapshot::Ref(_)
        | TimeoutSnapshot::PenaltyShot(_) => {
            center_col = center_col.push(
                make_button("END TIMEOUT")
                    .style(style::Button::Yellow)
                    .on_press(Message::EndTimeout),
            )
        }
        TimeoutSnapshot::None => {
            match snapshot.current_period {
                GamePeriod::BetweenGames
                | GamePeriod::HalfTime
                | GamePeriod::PreOvertime
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::PreSuddenDeath => {
                    center_col = center_col.push(
                        make_button("START NOW")
                            .style(style::Button::Green)
                            .on_press(Message::StartPlayNow),
                    )
                }
                GamePeriod::FirstHalf
                | GamePeriod::SecondHalf
                | GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::SuddenDeath => {}
            };
        }
    };

    center_col = center_col.push(
        button(
            text(config_string(snapshot, config, using_uwhscores, games))
                .size(SMALL_TEXT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Left),
        )
        .padding(PADDING)
        .style(style::Button::LightGray)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(Message::ShowGameDetails),
    );

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
            style::Button::Red
        } else {
            match color {
                GameColor::Black => style::Button::Black,
                GameColor::White => style::Button::White,
            }
        };

        button(
            column()
                .spacing(SPACING)
                .push(
                    text("Penalties")
                        .vertical_alignment(Vertical::Center)
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fill),
                )
                .push(
                    text(penalty_string(penalties))
                        .vertical_alignment(Vertical::Top)
                        .horizontal_alignment(Horizontal::Left)
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
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
        column()
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .push("BLACK")
            .push(text(snapshot.b_score.to_string()).size(LARGE_TEXT)),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Units(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(style::Button::Black);

    let mut black_new_score_btn = make_button("SCORE\nBLACK").style(style::Button::Black);

    let mut white_score_btn = button(
        column()
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .push("WHITE")
            .push(text(snapshot.w_score.to_string()).size(LARGE_TEXT)),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .height(Length::Units(MIN_BUTTON_SIZE + SMALL_PLUS_TEXT + PADDING))
    .style(style::Button::White);

    let mut white_new_score_btn = make_button("SCORE\nWHITE").style(style::Button::White);

    if snapshot.current_period != GamePeriod::BetweenGames {
        black_score_btn = black_score_btn.on_press(Message::EditScores);
        black_new_score_btn = black_new_score_btn.on_press(Message::AddNewScore(GameColor::Black));
        white_score_btn = white_score_btn.on_press(Message::EditScores);
        white_new_score_btn = white_new_score_btn.on_press(Message::AddNewScore(GameColor::White));
    }

    let black_col = column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .push(black_score_btn)
        .push(black_new_score_btn)
        .push(make_penalty_button(snapshot, GameColor::Black));

    let white_col = column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .push(white_score_btn)
        .push(white_new_score_btn)
        .push(make_penalty_button(snapshot, GameColor::White));

    row()
        .spacing(0)
        .height(Length::Fill)
        .push(
            row()
                .width(Length::Fill)
                .spacing(0)
                .push(black_col)
                .push(horizontal_space(Length::Units(3 * SPACING / 4))),
        )
        .push(
            row()
                .width(Length::FillPortion(2))
                .spacing(0)
                .push(horizontal_space(Length::Units(SPACING / 4)))
                .push(center_col)
                .push(horizontal_space(Length::Units(SPACING / 4))),
        )
        .push(
            row()
                .width(Length::Fill)
                .spacing(0)
                .push(horizontal_space(Length::Units(3 * SPACING / 4)))
                .push(white_col),
        )
        .into()
}
