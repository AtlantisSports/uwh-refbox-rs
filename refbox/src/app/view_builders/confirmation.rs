use super::*;
use iced::{
    Alignment, Element, Length, Theme,
    alignment::Horizontal,
    widget::{
        button::{Status, Style},
        column, container, horizontal_space, row, text, vertical_space,
    },
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

    let header_text: String = match kind {
        ConfirmationKind::GameConfigChanged(_) => fl!("game-configuration-can-not-be-changed"),
        ConfirmationKind::GameNumberChanged => fl!("apply-this-game-number-change"),
        ConfirmationKind::Error(string) => string.clone(),
        ConfirmationKind::UwhPortalIncomplete => fl!("UWHPortal-enabled"),
        ConfirmationKind::UwhPortalLinkFailed(PortalTokenResponse::InvalidCode) => {
            fl!("uwhportal-token-invalid-code")
        }
        ConfirmationKind::UwhPortalLinkFailed(PortalTokenResponse::NoPendingLink) => {
            fl!("uwhportal-token-no-pending-link")
        }
        ConfirmationKind::UwhPortalLinkFailed(PortalTokenResponse::Success(_)) => unreachable!(),
    };

    type ButtonStyleFn = fn(&Theme, Status) -> Style;

    let buttons: Vec<(_, ButtonStyleFn, _)> = match kind {
        ConfirmationKind::GameConfigChanged(_) => vec![
            (
                fl!("go-back-to-editor"),
                green_button,
                ConfirmationOption::GoBack,
            ),
            (
                fl!("discard-changes"),
                yellow_button,
                ConfirmationOption::DiscardChanges,
            ),
            (
                fl!("end-current-game-and-apply-changes"),
                red_button,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::GameNumberChanged => vec![
            (
                fl!("go-back-to-editor"),
                green_button,
                ConfirmationOption::GoBack,
            ),
            (
                fl!("discard-changes"),
                yellow_button,
                ConfirmationOption::DiscardChanges,
            ),
            (
                fl!("keep-current-game-and-apply-change"),
                orange_button,
                ConfirmationOption::KeepGameAndApply,
            ),
            (
                fl!("end-current-game-and-apply-change"),
                red_button,
                ConfirmationOption::EndGameAndApply,
            ),
        ],
        ConfirmationKind::Error(_) => {
            vec![(fl!("ok"), green_button, ConfirmationOption::DiscardChanges)]
        }
        ConfirmationKind::UwhPortalIncomplete => vec![
            (
                fl!("go-back-to-editor"),
                green_button,
                ConfirmationOption::GoBack,
            ),
            (
                fl!("discard-changes"),
                yellow_button,
                ConfirmationOption::DiscardChanges,
            ),
        ],
        ConfirmationKind::UwhPortalLinkFailed(_) => {
            vec![(fl!("ok"), green_button, ConfirmationOption::GoBack)]
        }
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
        vertical_space(),
        row![
            horizontal_space(),
            container(
                column![text(header_text).align_x(Horizontal::Center), button_col]
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            )
            .width(Length::FillPortion(3))
            .style(light_gray_container)
            .padding(PADDING),
            horizontal_space()
        ],
        vertical_space()
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .into()
}

pub(in super::super) fn build_score_confirmation_page<'a>(
    data: ViewData<'_, '_>,
    scores: BlackWhiteBundle<u8>,
    confirmation_time: Option<u32>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let time = time_string(Duration::from_secs(confirmation_time.unwrap_or(0) as u64));

    let header = text(fl!(
        "confirm-score",
        score_black = scores.black,
        score_white = scores.white,
        countdown = time
    ))
    .align_x(Horizontal::Center);

    let options = row![
        make_button(fl!("yes"))
            .style(green_button)
            .on_press(Message::ScoreConfirmation { correct: true }),
        make_button(fl!("no"))
            .style(red_button)
            .on_press(Message::ScoreConfirmation { correct: false }),
    ]
    .spacing(SPACING)
    .width(Length::Fill);

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        vertical_space(),
        row![
            horizontal_space(),
            container(
                column![header, options]
                    .spacing(SPACING)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            )
            .width(Length::FillPortion(3))
            .style(light_gray_container)
            .padding(PADDING),
            horizontal_space()
        ],
        vertical_space()
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .into()
}
