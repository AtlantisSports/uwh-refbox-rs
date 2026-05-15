use super::*;
use crate::portal_manager::HealthState;
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
        portal_indicator,
        ..
    } = data;

    let header_text: String = match kind {
        ConfirmationKind::GameConfigChangedFromApply(_) => {
            fl!("game-configuration-can-not-be-changed")
        }
        ConfirmationKind::GameNumberChangedFromApply => fl!("apply-this-game-number-change"),
        ConfirmationKind::Error(string) => string.clone(),
        ConfirmationKind::UwhPortalIncompleteFromApply => {
            fl!("portal-enabled", portal = portal_name_for_mode(mode))
        }
        ConfirmationKind::UwhPortalLinkFailed(PortalTokenResponse::InvalidCode) => {
            fl!("uwhportal-token-invalid-code")
        }
        ConfirmationKind::UwhPortalLinkFailed(PortalTokenResponse::NoPendingLink) => {
            fl!("uwhportal-token-no-pending-link")
        }
        ConfirmationKind::UwhPortalLinkFailed(PortalTokenResponse::Success(_)) => unreachable!(),
        ConfirmationKind::PortalTenantSwitch { from_mode, to_mode } => fl!(
            "mode-switch-portal-tenant",
            from_mode = format!("{from_mode}"),
            to_mode = format!("{to_mode}"),
            from_portal = portal_name_for_mode(*from_mode),
            to_portal = portal_name_for_mode(*to_mode)
        ),
    };

    type ButtonStyleFn = fn(&Theme, Status) -> Style;

    let buttons: Vec<(_, ButtonStyleFn, _)> = match kind {
        ConfirmationKind::GameConfigChangedFromApply(_) => vec![
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
        ConfirmationKind::GameNumberChangedFromApply => vec![
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
        ConfirmationKind::UwhPortalIncompleteFromApply => vec![
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
        ConfirmationKind::PortalTenantSwitch { .. } => vec![
            (
                fl!("cancel"),
                red_button,
                ConfirmationOption::DiscardChanges,
            ),
            (
                fl!("restart-to-apply"),
                blue_button,
                ConfirmationOption::RestartAndApply,
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
        make_game_time_button(snapshot, false, true, mode, clock_running, portal_indicator),
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
        portal_indicator,
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

    let mut body = column![].spacing(SPACING).width(Length::Fill);
    // When no portal event is linked, no submission-path advisory can
    // meaningfully apply, so the banner is suppressed entirely.
    if portal_indicator.is_some_and(|s| s.health == HealthState::Red) {
        body = body.push(
            container(
                text(fl!("portal-advisory-at-game-end"))
                    .style(white_text)
                    .size(SMALL_TEXT),
            )
            .width(Length::Fill)
            .padding(PADDING)
            .style(red_container),
        );
    }
    body = body.push(header);
    body = body.push(options);

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running, portal_indicator),
        vertical_space(),
        row![
            horizontal_space(),
            container(body.align_x(Alignment::Center))
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
