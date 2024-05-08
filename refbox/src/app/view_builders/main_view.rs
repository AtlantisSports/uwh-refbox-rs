use super::{
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
    config: &GameConfig,
    using_uwhscores: bool,
    games: &Option<BTreeMap<u32, GameInfo>>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let time_button = make_game_time_button(snapshot, true, false, mode, clock_running);

    let mut center_col = column![time_button].spacing(SPACING).width(Length::Fill);

    match snapshot.timeout {
        TimeoutSnapshot::White(_)
        | TimeoutSnapshot::Black(_)
        | TimeoutSnapshot::Ref(_)
        | TimeoutSnapshot::PenaltyShot(_) => {
            center_col = center_col.push(
                make_button(fl!("end-timeout"))
                    .style(ButtonStyle::Yellow)
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
                        make_button(fl!("start-now"))
                            .style(ButtonStyle::Green)
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
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Left),
        )
        .padding(PADDING)
        .style(ButtonStyle::LightGray)
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
