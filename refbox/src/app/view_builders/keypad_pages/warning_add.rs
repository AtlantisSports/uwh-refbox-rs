use super::*;
use iced::{
    Length, Theme,
    widget::{
        Space,
        button::{Status, Style},
        column, row, vertical_space,
    },
};
use uwh_common::color::Color as GameColor;

type StyleFn = fn(&Theme, Status) -> Style;

pub(super) fn make_warning_add_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    foul: Infraction,
    team_warning: bool,
    ret_to_overview: bool,
    player_num: u32,
) -> Element<'a, Message> {
    let (black_style, white_style): (StyleFn, StyleFn) = match color {
        GameColor::Black => (black_selected_button, white_button),
        GameColor::White => (black_button, white_selected_button),
    };

    let team_warning_style = if team_warning {
        blue_selected_button
    } else {
        blue_button
    };

    let mut exit_row = row![
        make_button(fl!("cancel"))
            .style(red_button)
            .width(Length::Fill)
            .on_press(Message::WarningEditComplete {
                canceled: true,
                deleted: false,
                ret_to_overview
            })
    ]
    .spacing(SPACING);

    if origin.is_some() {
        exit_row = exit_row.push(
            make_button(fl!("delete"))
                .style(orange_button)
                .width(Length::Fill)
                .on_press(Message::WarningEditComplete {
                    canceled: false,
                    deleted: true,
                    ret_to_overview,
                }),
        );
    }

    exit_row = exit_row.push(
        make_button(fl!("done"))
            .style(green_button)
            .width(Length::Fill)
            .on_press_maybe(
                warning_add_can_commit(foul, team_warning, player_num).then_some(
                    Message::WarningEditComplete {
                        canceled: false,
                        deleted: false,
                        ret_to_overview,
                    },
                ),
            ),
    );
    column![
        row![
            make_multi_label_button((fl!("team-warning-line-1"), fl!("team-warning-line-2")))
                .on_press(Message::ToggleBoolParameter(BoolGameParameter::TeamWarning))
                .style(team_warning_style),
            make_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            make_button(fl!("light-team-name-caps"))
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        Space::with_height(SPACING),
        make_penalty_dropdown(foul, true),
        vertical_space(),
        exit_row,
    ]
    .into()
}

/// Returns true when the warning entry can be saved: an infraction must always
/// be selected, and an individual warning also needs a player number. A team
/// warning (`team_warning == true`) has no player, so it needs only the infraction.
fn warning_add_can_commit(infraction: Infraction, team_warning: bool, player_num: u32) -> bool {
    !matches!(infraction, Infraction::Unknown) && (team_warning || player_num > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_needs_infraction() {
        assert!(!warning_add_can_commit(Infraction::Unknown, true, 0));
    }

    #[test]
    fn warning_team_with_infraction_ok_without_number() {
        assert!(warning_add_can_commit(
            Infraction::StickInfringement,
            true,
            0
        ));
    }

    #[test]
    fn warning_individual_needs_number() {
        assert!(!warning_add_can_commit(
            Infraction::StickInfringement,
            false,
            0
        ));
        assert!(warning_add_can_commit(
            Infraction::StickInfringement,
            false,
            7
        ));
    }

    #[test]
    fn warning_individual_with_number_still_needs_infraction() {
        assert!(!warning_add_can_commit(Infraction::Unknown, false, 7));
    }
}
