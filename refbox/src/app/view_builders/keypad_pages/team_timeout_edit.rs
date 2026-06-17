use super::*;
use iced::{
    Length, Theme,
    widget::{
        button::{Status, Style},
        column, row, text, vertical_space,
    },
};
use std::time::Duration;

type StyleFn = fn(&Theme, Status) -> Style;

/// Length presets shown on the team-timeout edit page: (label, seconds).
const LENGTH_PRESETS: [(&str, u64); 5] = [
    ("0:30", 30),
    ("0:45", 45),
    ("1:00", 60),
    ("1:15", 75),
    ("1:30", 90),
];

pub(super) fn make_team_timeout_edit_page<'a>(
    duration: Duration,
    timeouts_counted_per_half: bool,
    count: u32,
) -> Element<'a, Message> {
    // Count 0 means "no team timeouts": the period and length controls are
    // meaningless, so they render disabled (greyed, non-pressable). The count
    // buttons themselves stay active. Any non-zero count shows "1" selected.
    let zero_selected = count == 0;
    let count_enabled = !zero_selected;

    let (zero_style, zero_msg): (StyleFn, _) = if zero_selected {
        (blue_selected_button, Message::NoAction)
    } else {
        (blue_button, Message::SetTeamTimeoutCount(0))
    };
    let (one_style, one_msg): (StyleFn, _) = if zero_selected {
        (blue_button, Message::SetTeamTimeoutCount(1))
    } else {
        (blue_selected_button, Message::NoAction)
    };

    let count_row = row![
        text(fl!("team-timeout-count"))
            .size(SMALL_PLUS_TEXT)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .align_y(Vertical::Center),
        make_button("0")
            .style(zero_style)
            .width(Length::Fill)
            .on_press(zero_msg),
        make_button("1")
            .style(one_style)
            .width(Length::Fill)
            .on_press(one_msg),
    ]
    .spacing(SPACING);

    // HALF/GAME toggle. Styles always reflect the current selection so the
    // operator can still see the chosen period while disabled; on_press is
    // only attached when the count is non-zero.
    let half_style: StyleFn = if timeouts_counted_per_half {
        blue_selected_button
    } else {
        blue_button
    };
    let game_style: StyleFn = if timeouts_counted_per_half {
        blue_button
    } else {
        blue_selected_button
    };
    let (half_msg, game_msg) = if timeouts_counted_per_half {
        (
            Message::NoAction,
            Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
        )
    } else {
        (
            Message::ToggleBoolParameter(BoolGameParameter::TimeoutsCountedPerHalf),
            Message::NoAction,
        )
    };
    let mut half_button = make_button(fl!("half"))
        .style(half_style)
        .width(Length::Fill);
    let mut game_button = make_button(fl!("game"))
        .style(game_style)
        .width(Length::Fill);
    if count_enabled {
        half_button = half_button.on_press(half_msg);
        game_button = game_button.on_press(game_msg);
    }

    let counted_per_row = row![
        text(fl!("timeouts-counted-per"))
            .size(SMALL_PLUS_TEXT)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .align_y(Vertical::Center),
        half_button,
        game_button,
    ]
    .spacing(SPACING);

    // Length presets. Selected = the preset matching the current duration.
    let make_preset = |label: &'a str, secs: u64| -> Element<'a, Message> {
        let preset_dur = Duration::from_secs(secs);
        let selected = duration == preset_dur;
        let style: StyleFn = if selected {
            blue_selected_button
        } else {
            blue_button
        };
        let mut b = make_button(label).style(style).width(Length::Fill);
        if count_enabled {
            b = b.on_press(if selected {
                Message::NoAction
            } else {
                Message::SetTeamTimeoutLength(preset_dur)
            });
        }
        b.into()
    };

    let presets_row = row![
        make_preset(LENGTH_PRESETS[0].0, LENGTH_PRESETS[0].1),
        make_preset(LENGTH_PRESETS[1].0, LENGTH_PRESETS[1].1),
        make_preset(LENGTH_PRESETS[2].0, LENGTH_PRESETS[2].1),
        make_preset(LENGTH_PRESETS[3].0, LENGTH_PRESETS[3].1),
        make_preset(LENGTH_PRESETS[4].0, LENGTH_PRESETS[4].1),
    ]
    .spacing(SPACING);

    let length_block = column![
        text(fl!("timeout-length"))
            .size(SMALL_PLUS_TEXT)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .align_y(Vertical::Center),
        presets_row,
    ]
    .spacing(SPACING);

    column![
        count_row,
        counted_per_row,
        vertical_space(),
        length_block,
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            make_button(fl!("apply"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
