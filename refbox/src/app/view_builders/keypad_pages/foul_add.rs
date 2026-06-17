use super::*;
use iced::{
    Length, Theme,
    widget::{
        Space,
        button::{Status, Style},
        column, row, vertical_space,
    },
};

type StyleFn = fn(&Theme, Status) -> Style;

use uwh_common::color::Color as GameColor;

pub(super) fn make_foul_add_page<'a>(
    origin: Option<(Option<GameColor>, usize)>,
    color: Option<GameColor>,
    foul: Infraction,
    ret_to_overview: bool,
    player_num: u32,
) -> Element<'a, Message> {
    let (black_style, white_style, equal_style): (StyleFn, StyleFn, StyleFn) = match color {
        Some(GameColor::Black) => (black_selected_button, white_button, blue_button),
        Some(GameColor::White) => (black_button, white_selected_button, blue_button),
        None => (black_button, white_button, blue_selected_button),
    };

    let mut exit_row = row![
        make_button(fl!("cancel"))
            .style(red_button)
            .width(Length::Fill)
            .on_press(Message::FoulEditComplete {
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
                .on_press(Message::FoulEditComplete {
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
            .on_press_maybe(foul_add_can_commit(foul, color, player_num).then_some(
                Message::FoulEditComplete {
                    canceled: false,
                    deleted: false,
                    ret_to_overview,
                },
            )),
    );
    column![
        row![
            make_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            button(centered_text("=").size(LARGE_TEXT))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .width(Length::Fill)
                .on_press(Message::ChangeColor(None))
                .style(equal_style),
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

/// Returns true when the foul entry has everything it needs to be saved: an
/// infraction must always be selected, and an individual foul (Black/White)
/// also needs a player number. An "equal" foul (`color == None`) has no player,
/// so it needs only the infraction.
fn foul_add_can_commit(infraction: Infraction, color: Option<GameColor>, player_num: u32) -> bool {
    !matches!(infraction, Infraction::Unknown) && (color.is_none() || player_num > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foul_needs_infraction() {
        // Equal foul, infraction unset → blocked.
        assert!(!foul_add_can_commit(Infraction::Unknown, None, 0));
    }

    #[test]
    fn foul_equal_with_infraction_ok_without_number() {
        assert!(foul_add_can_commit(Infraction::StickInfringement, None, 0));
    }

    #[test]
    fn foul_individual_needs_number() {
        assert!(!foul_add_can_commit(
            Infraction::StickInfringement,
            Some(GameColor::Black),
            0
        ));
        assert!(foul_add_can_commit(
            Infraction::StickInfringement,
            Some(GameColor::Black),
            5
        ));
    }

    #[test]
    fn foul_individual_with_number_still_needs_infraction() {
        assert!(!foul_add_can_commit(
            Infraction::Unknown,
            Some(GameColor::Black),
            5
        ));
    }
}
