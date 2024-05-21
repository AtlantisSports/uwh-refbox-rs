use super::{
    fl,
    style::{ButtonStyle, ContainerStyle, Element, LINE_HEIGHT, PADDING, SPACING},
    *,
};

use iced::{
    alignment::Horizontal,
    widget::{column, container, horizontal_space, row, text, vertical_space},
    Alignment, Length,
};

use uwh_common::game_snapshot::GameSnapshot;

pub(in super::super) fn build_confirmation_page<'a>(
    snapshot: &GameSnapshot,
    kind: &ConfirmationKind,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let header_text: String = match kind {
        ConfirmationKind::GameConfigChanged(_) => fl!("game-configuration-can-not-be-changed"),
        ConfirmationKind::GameNumberChanged => fl!("apply-this-game-number-change"),
        ConfirmationKind::Error(string) => string.clone(),
        ConfirmationKind::UwhScoresIncomplete => fl!("UWHScores-enabled"),
    };

    let buttons = match kind {
        ConfirmationKind::GameConfigChanged(_) => vec![
            (
                fl!("go-back-to-editor"),
                ButtonStyle::Green,
                ConfirmationOption::GoBack,
            ),
            (
                fl!("discard-changes"),
                ButtonStyle::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                fl!("end-current-game-and-apply-changes"),
                ButtonStyle::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::GameNumberChanged => vec![
            (
                fl!("go-back-to-editor"),
                ButtonStyle::Green,
                ConfirmationOption::GoBack,
            ),
            (
                fl!("discard-changes"),
                ButtonStyle::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                fl!("keep-current-game-and-apply-change"),
                ButtonStyle::Orange,
                ConfirmationOption::KeepGameAndApply,
            ),
            (
                fl!("end-current-game-and-apply-change"),
                ButtonStyle::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::Error(_) => {
            vec![(
                fl!("ok"),
                ButtonStyle::Green,
                ConfirmationOption::DiscardChanges,
            )]
        }
        ConfirmationKind::UwhScoresIncomplete => vec![
            (
                fl!("go-back-to-editor"),
                ButtonStyle::Green,
                ConfirmationOption::GoBack,
            ),
            (
                fl!("discard-changes"),
                ButtonStyle::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
        ],
    };

    let buttons = buttons.into_iter().map(|(text, style, option)| {
        make_button(text)
            .style(style)
            .on_press(Message::ConfirmationSelected(option))
    });

    let mut button_col = column![].spacing(SPACING).width(Length::Fill);

    for button in buttons {
        button_col = button_col.push(button);
    }

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            container(
                column![
                    text(header_text)
                        .line_height(LINE_HEIGHT)
                        .horizontal_alignment(Horizontal::Center),
                    button_col
                ]
                .spacing(SPACING)
                .width(Length::Fill)
                .align_items(Alignment::Center),
            )
            .width(Length::FillPortion(3))
            .style(ContainerStyle::LightGray)
            .padding(PADDING),
            horizontal_space(Length::Fill)
        ],
        vertical_space(Length::Fill)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .align_items(Alignment::Center)
    .into()
}

pub(in super::super) fn build_score_confirmation_page<'a>(
    snapshot: &GameSnapshot,
    scores: BlackWhiteBundle<u8>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let header = text(fl!(
        "confirm-score",
        score_black = scores.black,
        score_white = scores.white
    ))
    .line_height(LINE_HEIGHT)
    .horizontal_alignment(Horizontal::Center);

    let options = row![
        make_button(fl!("yes"))
            .style(ButtonStyle::Green)
            .on_press(Message::ScoreConfirmation { correct: true }),
        make_button(fl!("no"))
            .style(ButtonStyle::Red)
            .on_press(Message::ScoreConfirmation { correct: false }),
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            container(
                column![header, options]
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_items(Alignment::Center),
            )
            .width(Length::FillPortion(3))
            .style(ContainerStyle::LightGray)
            .padding(PADDING),
            horizontal_space(Length::Fill)
        ],
        vertical_space(Length::Fill)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .align_items(Alignment::Center)
    .into()
}
