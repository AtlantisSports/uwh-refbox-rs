use super::{style::Element, *};
use iced::{
    widget::{column, row, vertical_space},
    Length,
};
use uwh_common::game_snapshot::Color as GameColor;

pub(super) fn make_penalty_edit_page<'a>(
    origin: Option<(GameColor, usize)>,
    color: GameColor,
    kind: PenaltyKind,
    config: &Config,
    infraction: Infraction,
) -> Element<'a, Message> {
    let (black_style, white_style) = match color {
        GameColor::Black => (ButtonStyle::BlackSelected, ButtonStyle::White),
        GameColor::White => (ButtonStyle::Black, ButtonStyle::WhiteSelected),
    };

    let (green, yellow, orange) = match config.mode {
        Mode::Hockey6V6 => (
            PenaltyKind::OneMinute,
            PenaltyKind::TwoMinute,
            PenaltyKind::FiveMinute,
        ),

        Mode::Hockey3V3 => (
            PenaltyKind::ThirtySecond,
            PenaltyKind::OneMinute,
            PenaltyKind::TwoMinute,
        ),

        Mode::Rugby => (
            PenaltyKind::TwoMinute,
            PenaltyKind::FourMinute,
            PenaltyKind::FiveMinute,
        ),
    };

    let (green_style, yellow_style, orange_style, td_style) = if kind == green {
        (
            ButtonStyle::GreenSelected,
            ButtonStyle::Yellow,
            ButtonStyle::Orange,
            ButtonStyle::Red,
        )
    } else if kind == yellow {
        (
            ButtonStyle::Green,
            ButtonStyle::YellowSelected,
            ButtonStyle::Orange,
            ButtonStyle::Red,
        )
    } else if kind == orange {
        (
            ButtonStyle::Green,
            ButtonStyle::Yellow,
            ButtonStyle::OrangeSelected,
            ButtonStyle::Red,
        )
    } else if kind == PenaltyKind::TotalDismissal {
        (
            ButtonStyle::Green,
            ButtonStyle::Yellow,
            ButtonStyle::Orange,
            ButtonStyle::RedSelected,
        )
    } else {
        (
            ButtonStyle::Green,
            ButtonStyle::Yellow,
            ButtonStyle::Orange,
            ButtonStyle::Red,
        )
    };

    let mut exit_row = row![make_smaller_button("CANCEL")
        .style(ButtonStyle::Red)
        .width(Length::Fill)
        .on_press(Message::PenaltyEditComplete {
            canceled: true,
            deleted: false,
        }),]
    .spacing(SPACING);

    if origin.is_some() {
        exit_row = exit_row.push(
            make_smaller_button("DELETE")
                .style(ButtonStyle::Orange)
                .width(Length::Fill)
                .on_press(Message::PenaltyEditComplete {
                    canceled: false,
                    deleted: true,
                }),
        );
    }

    exit_row = exit_row.push(
        make_smaller_button("DONE")
            .style(ButtonStyle::Green)
            .width(Length::Fill)
            .on_press(Message::PenaltyEditComplete {
                canceled: false,
                deleted: false,
            }),
    );

    let labels: Vec<&str> = [green, yellow, orange]
        .iter()
        .map(|kind| match kind {
            PenaltyKind::ThirtySecond => "30s",
            PenaltyKind::OneMinute => "1m",
            PenaltyKind::TwoMinute => "2m",
            PenaltyKind::FourMinute => "4m",
            PenaltyKind::FiveMinute => "5m",
            PenaltyKind::TotalDismissal => "TD",
        })
        .collect();

    let green_label = labels[0];
    let yellow_label = labels[1];
    let orange_label = labels[2];

    let mut content = column![row![
        make_smaller_button("BLACK")
            .style(black_style)
            .on_press(Message::ChangeColor(Some(GameColor::Black))),
        make_smaller_button("WHITE")
            .style(white_style)
            .on_press(Message::ChangeColor(Some(GameColor::White))),
    ]
    .spacing(SPACING)]
    .spacing(SPACING);

    if config.track_fouls_and_warnings {
        content = content.push(make_penalty_dropdown(infraction));
    } else {
        content = content.push(vertical_space(Length::Fill));
    }

    content = content.push(
        row![
            make_smaller_button(green_label)
                .style(green_style)
                .on_press(Message::ChangeKind(green)),
            make_smaller_button(yellow_label)
                .style(yellow_style)
                .on_press(Message::ChangeKind(yellow)),
            make_smaller_button(orange_label)
                .style(orange_style)
                .on_press(Message::ChangeKind(orange)),
            make_smaller_button("TD")
                .style(td_style)
                .on_press(Message::ChangeKind(PenaltyKind::TotalDismissal)),
        ]
        .spacing(SPACING),
    );

    if !config.track_fouls_and_warnings {
        content = content.push(vertical_space(Length::Fill));
    }

    content = content.push(exit_row);
    content.into()
}
