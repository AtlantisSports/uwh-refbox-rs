use super::{
    style::{ButtonStyle, ContainerStyle, Element, LINE_HEIGHT, PADDING, SPACING},
    *,
};

use iced::{
    Alignment, Length,
    alignment::Horizontal,
    widget::{column, container, horizontal_space, row, text, vertical_space},
};

pub(in super::super) fn build_confirmation_page<'a>(
    data: ViewData<'_, '_>,
    kind: &ConfirmationKind,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let header_text = match kind {
        ConfirmationKind::GameConfigChanged(_) => {
            "The game configuration can not be changed while a game is in progress.\n\nWhat would you like to do?"
        }
        ConfirmationKind::GameNumberChanged => {
            "How would you like to apply this game number change?"
        }
        ConfirmationKind::Error(string) => string,
        ConfirmationKind::UwhPortalIncomplete => {
            "When UWHPortal is enabled, all fields must be filled out."
        }
    };

    let buttons = match kind {
        ConfirmationKind::GameConfigChanged(_) => vec![
            (
                "GO BACK TO EDITOR",
                ButtonStyle::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                ButtonStyle::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                "END CURRENT GAME AND APPLY CHANGES",
                ButtonStyle::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::GameNumberChanged => vec![
            (
                "GO BACK TO EDITOR",
                ButtonStyle::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
                ButtonStyle::Yellow,
                ConfirmationOption::DiscardChanges,
            ),
            (
                "KEEP CURRENT GAME AND APPLY CHANGE",
                ButtonStyle::Orange,
                ConfirmationOption::KeepGameAndApply,
            ),
            (
                "END CURRENT GAME AND APPLY CHANGE",
                ButtonStyle::Red,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::Error(_) => {
            vec![("OK", ButtonStyle::Green, ConfirmationOption::DiscardChanges)]
        }
        ConfirmationKind::UwhPortalIncomplete => vec![
            (
                "GO BACK TO EDITOR",
                ButtonStyle::Green,
                ConfirmationOption::GoBack,
            ),
            (
                "DISCARD CHANGES",
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
    data: ViewData<'_, '_>,
    scores: BlackWhiteBundle<u8>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let header = text(format!(
        "Is this score correct?\nConfirm with cheif referee.\n\nBlack: {}        White: {}\n",
        scores.black, scores.white
    ))
    .line_height(LINE_HEIGHT)
    .horizontal_alignment(Horizontal::Center);

    let options = row![
        make_button("YES")
            .style(ButtonStyle::Green)
            .on_press(Message::ScoreConfirmation { correct: true }),
        make_button("NO")
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
