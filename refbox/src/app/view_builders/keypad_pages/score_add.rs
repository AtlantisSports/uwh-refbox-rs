use super::*;
use iced::{
    Length, Theme,
    widget::{
        button::{Status, Style},
        column, row, vertical_space,
    },
};
use uwh_common::color::Color as GameColor;

type StyleFn = fn(&Theme, Status) -> Style;

pub(super) fn make_score_add_page<'a>(
    color: GameColor,
    team_score: bool,
    player_num: u32,
) -> Element<'a, Message> {
    let (black_style, white_style): (StyleFn, StyleFn) = match color {
        GameColor::Black => (black_selected_button, white_button),
        GameColor::White => (black_button, white_selected_button),
    };

    let team_score_style = if team_score {
        blue_selected_button
    } else {
        blue_button
    };

    column![
        // The team option sits between the two teams, matching the foul page's
        // "=" button, and the row is pinned to the top like the warnings page's.
        row![
            make_button(fl!("dark-team-name-caps"))
                .style(black_style)
                .on_press(Message::ChangeColor(Some(GameColor::Black))),
            make_multi_label_button((fl!("team-score-line-1"), fl!("team-score-line-2")))
                .on_press(Message::ToggleBoolParameter(BoolGameParameter::TeamScore))
                .style(team_score_style),
            make_button(fl!("light-team-name-caps"))
                .style(white_style)
                .on_press(Message::ChangeColor(Some(GameColor::White))),
        ]
        .spacing(SPACING),
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::AddScoreComplete { canceled: true }),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press_maybe(
                    score_add_can_commit(team_score, player_num)
                        .then_some(Message::AddScoreComplete { canceled: false }),
                ),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}

/// Returns true when the goal can be saved: either a scorer is named, or the
/// operator has explicitly attributed it to the team — an unknown scorer, or a
/// penalty goal where no individual is credited.
///
/// Not default-true. Previously DONE committed a team goal whenever no number
/// was selected, so a forgotten scorer was silently recorded as a team goal.
/// Attribution now costs one deliberate tap.
fn score_add_can_commit(team_score: bool, player_num: u32) -> bool {
    team_score || player_num > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_needs_a_player_or_an_explicit_team_choice() {
        // Nothing chosen — the old silent-team-goal case, now blocked.
        assert!(!score_add_can_commit(false, 0));
        // Explicitly the team's goal.
        assert!(score_add_can_commit(true, 0));
        // A named scorer.
        assert!(score_add_can_commit(false, 7));
    }
}
