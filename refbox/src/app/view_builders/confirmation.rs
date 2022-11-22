use super::{
    style::{self, PADDING, SPACING},
    *,
};

use iced::{
    alignment::Horizontal,
    pure::{column, container, horizontal_space, row, text, vertical_space, Element},
    Alignment, Length,
};

use uwh_common::game_snapshot::GameSnapshot;

pub(in super::super) fn build_confirmation_page<'a>(
    snapshot: &GameSnapshot,
    kind: &ConfirmationKind,
) -> Element<'a, Message> {
    let header_text = match kind {
        ConfirmationKind::GameConfigChanged => "The game configuration can not be changed while a game is in progress.\n\nWhat would you like to do?",
        ConfirmationKind::GameNumberChanged => "How would you like to apply this game number change?",
        ConfirmationKind::Error(string) => string,
        ConfirmationKind::UwhScoresIncomplete => "When UWHScores is enabled, all fields must be filled out."
            };

    let buttons = match kind {
        ConfirmationKind::GameConfigChanged => vec![
            (
                "GO BACK TO EDITOR",
                style::Button::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                style::Button::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                "END CURRENT GAME AND APPLY CHANGES",
                style::Button::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::GameNumberChanged => vec![
            (
                "GO BACK TO EDITOR",
                style::Button::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                style::Button::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                "KEEP CURRENT GAME AND APPLY CHANGE",
                style::Button::Orange,
                ConfirmationOption::KeepGameAndApply,
            ),
            (
                "END CURRENT GAME AND APPLY CHANGE",
                style::Button::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::Error(_) => {
            vec![(
                "OK",
                style::Button::Green,
                ConfirmationOption::DiscardChanges,
            )]
        }
        ConfirmationKind::UwhScoresIncomplete => vec![
            (
                "GO BACK TO EDITOR",
                style::Button::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                style::Button::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
        ],
    };

    let buttons = buttons.into_iter().map(|(text, style, option)| {
        make_button(text)
            .style(style)
            .on_press(Message::ConfirmationSelected(option))
    });

    let mut button_col = column().spacing(SPACING).width(Length::Fill);

    for button in buttons {
        button_col = button_col.push(button);
    }

    column()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .push(horizontal_space(Length::Fill))
                .push(
                    container(
                        column()
                            .spacing(SPACING)
                            .width(Length::Fill)
                            .align_items(Alignment::Center)
                            .push(text(header_text).horizontal_alignment(Horizontal::Center))
                            .push(button_col),
                    )
                    .width(Length::FillPortion(3))
                    .style(style::Container::LightGray)
                    .padding(PADDING),
                )
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .into()
}

pub(in super::super) fn build_score_confirmation_page<'a>(
    snapshot: &GameSnapshot,
    scores: BlackWhiteBundle<u8>,
) -> Element<'a, Message> {
    let header = text(format!(
        "Is this score correct?\n\nBlack: {}        White: {}\n",
        scores.black, scores.white
    ))
    .horizontal_alignment(Horizontal::Center);

    let options = row()
        .spacing(SPACING)
        .width(Length::Fill)
        .push(
            make_button("YES")
                .style(style::Button::Green)
                .on_press(Message::ScoreConfirmation { correct: true }),
        )
        .push(
            make_button("NO")
                .style(style::Button::Red)
                .on_press(Message::ScoreConfirmation { correct: false }),
        );

    column()
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .push(horizontal_space(Length::Fill))
                .push(
                    container(
                        column()
                            .spacing(SPACING)
                            .width(Length::Fill)
                            .align_items(Alignment::Center)
                            .push(header)
                            .push(options),
                    )
                    .width(Length::FillPortion(3))
                    .style(style::Container::LightGray)
                    .padding(PADDING),
                )
                .push(horizontal_space(Length::Fill)),
        )
        .push(vertical_space(Length::Fill))
        .into()
}
