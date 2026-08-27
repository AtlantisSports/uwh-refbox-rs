use super::*;
use iced::{
    Length,
    alignment::Vertical,
    widget::{button, column, container, horizontal_space, row},
};
use uwh_common::{
    bundles::BlackWhiteBundle, config::Game as GameConfig, uwhportal::schedule::Schedule,
};

pub(in super::super) fn build_game_info_page<'a>(
    data: ViewData<'_, '_>,
    config: &GameConfig,
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

    // A refresh re-fetches the schedule from the portal. Grey the button out
    // only when the saved login token is expired — the one case a re-login
    // fixes — and leave the operator to re-login via the connection
    // indicator. Every other Red cause leaves `token_expired` false and keeps
    // refresh usable: a stuck submission, a connectivity drop, or a startup
    // failure. In that last case there is no client to fetch with, so the
    // button stays live but its handler declines to spin (see
    // `Message::RequestPortalRefresh`).
    let token_expired = portal_indicator.is_some_and(|s| s.token_expired);

    let middle_item: Element<_> = if uses_remote {
        if is_refreshing {
            make_chrome_button(fl!("refreshing"))
                .style(blue_button)
                .width(Length::Fill)
                .into()
        } else {
            make_chrome_button(fl!("refresh"))
                .style(blue_button)
                .width(Length::Fill)
                .on_press_maybe((!token_expired).then_some(Message::RequestPortalRefresh))
                .into()
        }
    } else {
        horizontal_space().into()
    };

    use super::game_info_table::{game_info_rows, render_game_info_table};
    let table = render_game_info_table(game_info_rows(
        snapshot,
        config,
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
