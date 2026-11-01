use super::*;
use iced::{
    Length,
    alignment::Vertical,
    widget::{button, column, container, horizontal_space, row},
};
use uwh_common::{
    bundles::BlackWhiteBundle, config::Game as GameConfig, uwhportal::schedule::Schedule,
};

/// Which button sits between BACK and SETTINGS on the Game Info page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MiddleButton {
    /// No portal schedule in use, so there is nothing to refresh or restart from.
    Hidden,
    /// A refresh is already in flight.
    Refreshing,
    /// Between games: re-fetch the schedule from the portal.
    Refresh,
    /// A game is in play (any period other than between games): abandon it and put
    /// the court back between games, ready to start it again.
    Restart,
}

/// Choose the middle button. Kept pure so the rule is tested without building an app
/// or a view: a test cannot observe which message a rendered button would send.
fn middle_button(uses_remote: bool, is_refreshing: bool, period: GamePeriod) -> MiddleButton {
    if !uses_remote {
        MiddleButton::Hidden
    } else if is_refreshing {
        MiddleButton::Refreshing
    } else if period == GamePeriod::BetweenGames {
        MiddleButton::Refresh
    } else {
        MiddleButton::Restart
    }
}

pub(in super::super) fn build_game_info_page<'a>(
    data: ViewData<'_, '_>,
    config: &GameConfig,
    next_game_block: Duration,
    uses_remote: bool,
    is_refreshing: bool,
    schedule: Option<&Schedule>,
    last_game: Option<(GameNumber, BlackWhiteBundle<u8>)>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        teams,
        portal_indicator,
        ..
    } = data;

    // A refresh re-fetches the schedule from the portal, and a restart re-reads the
    // game's timing from it, so both share one rule. Grey the button out
    // only when the saved login token is expired — the one case a re-login
    // fixes — and leave the operator to re-login via the connection
    // indicator. Every other Red cause leaves `token_expired` false and keeps
    // refresh usable: a stuck submission, a connectivity drop, or a startup
    // failure. In that last case there is no client to fetch with, so the
    // button stays live but its handler declines to spin (see
    // `Message::RequestPortalRefresh`).
    let token_expired = portal_indicator.is_some_and(|s| s.token_expired);

    let middle_item: Element<_> =
        match middle_button(uses_remote, is_refreshing, snapshot.current_period) {
            MiddleButton::Hidden => horizontal_space().into(),
            MiddleButton::Refreshing => make_chrome_button(fl!("refreshing"))
                .style(blue_button)
                .width(Length::Fill)
                .into(),
            MiddleButton::Refresh => make_chrome_button(fl!("refresh"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press_maybe((!token_expired).then_some(Message::RequestPortalRefresh))
                .into(),
            // Task 3 points this at its own `RequestGameRestart` message.
            MiddleButton::Restart => make_chrome_button(fl!("restart"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press_maybe((!token_expired).then_some(Message::RequestPortalRefresh))
                .into(),
        };

    use super::game_info_table::{game_info_rows, render_game_info_table};
    let table = render_game_info_table(game_info_rows(
        snapshot,
        config,
        next_game_block,
        uses_remote,
        schedule,
        teams,
        last_game,
    ));
    // Pin the table to the top of the button via a transparent fill-height
    // wrapper, so the table's dark gridline backing only covers the grid and the
    // button's own colour shows through the empty space below it.
    let table_button = button(
        container(table)
            .align_y(Vertical::Top)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding(PADDING)
    .style(light_gray_button)
    .width(Length::Fill)
    .height(Length::Fill)
    .on_press(Message::EditGameConfigPage(ConfigPage::Game));

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        table_button,
        row![
            make_chrome_button(fl!("back"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete),
            middle_item,
            make_chrome_button(fl!("settings"))
                .style(gray_button)
                .width(Length::Fill)
                .on_press(Message::EditGameConfig),
        ]
        .spacing(SPACING)
        .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_button_rules() {
        use GamePeriod::*;
        use MiddleButton as B;

        let mut cases = vec![
            (false, false, BetweenGames, B::Hidden),
            (false, true, BetweenGames, B::Hidden),
            (false, false, FirstHalf, B::Hidden),
            (false, true, SuddenDeath, B::Hidden),
            (true, true, FirstHalf, B::Refreshing),
            (true, false, BetweenGames, B::Refresh),
        ];
        for period in [
            FirstHalf,
            HalfTime,
            SecondHalf,
            PreOvertime,
            OvertimeFirstHalf,
            OvertimeHalfTime,
            OvertimeSecondHalf,
            PreSuddenDeath,
            SuddenDeath,
        ] {
            cases.push((true, false, period, B::Restart));
        }

        for (uses_remote, is_refreshing, period, expected) in cases {
            assert_eq!(
                middle_button(uses_remote, is_refreshing, period),
                expected,
                "uses_remote={uses_remote}, is_refreshing={is_refreshing}, period={period:?}"
            );
        }
    }
}
