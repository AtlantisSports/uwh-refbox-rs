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
        make_chrome_button(fl!("cancel"))
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
            make_chrome_button(fl!("delete"))
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
        make_chrome_button(fl!("done"))
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
            make_chrome_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            button(centered_text("=").size(LARGE_TEXT))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .width(Length::Fill)
                .on_press(Message::ChangeColor(None))
                .style(equal_style),
            make_chrome_button(fl!("light-team-name-caps"))
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
/// infraction must always be selected. A player number is never required: a
/// Black/White foul left blank means the player was not recorded, and an
/// "equal" foul (`color == None`) has no player at all.
///
/// The team and player number stay in the signature even though the rule ignores
/// them, so `foul_gate_depends_only_on_the_infraction` can pin their irrelevance:
/// reintroducing either fails a test rather than silently re-blocking the operator.
fn foul_add_can_commit(
    infraction: Infraction,
    _color: Option<GameColor>,
    _player_num: u32,
) -> bool {
    !matches!(infraction, Infraction::Unknown)
}

/// The player a foul is saved against: none for an "equal" foul, and none when
/// the number was left blank (0) on a Black/White foul -- never player 0.
pub(crate) fn foul_player_number(color: Option<GameColor>, player_num: u32) -> Option<u8> {
    if color.is_none() || player_num == 0 {
        None
    } else {
        // Cannot panic: the foul keypad caps the number at 99 (`KeypadPage::max_val`).
        Some(player_num.try_into().unwrap())
    }
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
    fn foul_individual_number_is_optional() {
        assert!(foul_add_can_commit(
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
    fn foul_individual_without_number_still_needs_infraction() {
        assert!(!foul_add_can_commit(
            Infraction::Unknown,
            Some(GameColor::White),
            0
        ));
    }

    #[test]
    fn foul_gate_depends_only_on_the_infraction() {
        for color in [None, Some(GameColor::Black), Some(GameColor::White)] {
            for num in [0, 7, 99] {
                assert!(foul_add_can_commit(Infraction::Obstruction, color, num));
                assert!(!foul_add_can_commit(Infraction::Unknown, color, num));
            }
        }
    }

    #[test]
    fn foul_blank_number_is_no_player() {
        assert_eq!(foul_player_number(Some(GameColor::Black), 0), None);
        assert_eq!(foul_player_number(Some(GameColor::White), 7), Some(7));
        assert_eq!(foul_player_number(None, 0), None);
        assert_eq!(foul_player_number(None, 7), None);
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
