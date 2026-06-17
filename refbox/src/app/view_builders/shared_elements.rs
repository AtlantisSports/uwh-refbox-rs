use super::*;
use crate::app::RevivePhase;
use crate::portal_manager::{HealthState, PortalIndicatorState};
use enum_iterator::all;
use iced::{
    Alignment, Background, Border, Length, Theme,
    alignment::{Horizontal, Vertical},
    widget::{
        Button, Container, Image, Row, Space, Text, button, container,
        container::Style as ContainerStyle, horizontal_space, image, mouse_area, svg, svg::Svg,
        text, text::Style as TextStyle, vertical_space,
    },
};
use iced_core::border::Radius;
use iced_core::text::IntoFragment;
use matrix_drawing::{secs_to_long_time_string, secs_to_time_string};
use std::{
    fmt::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use uwh_common::{
    color::Color as GameColor,
    config::Game as GameConfig,
    game_snapshot::{
        GamePeriod, GameSnapshot, Infraction, InfractionSnapshot, PenaltySnapshot, PenaltyTime,
        TimeoutSnapshot,
    },
    uwhportal::schedule::{Game, GameList, ResultOf, Schedule, ScheduledTeam, TeamList},
};

macro_rules! column {
    () => (
        iced::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Column::with_children(vec![$($crate::app::Element::from($x)),+])
    );
}

macro_rules! row {
    () => (
        iced::widget::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        iced::widget::Row::with_children(vec![$($crate::app::Element::from($x)),+])
    );
}

pub(super) fn make_scroll_list<'a, const LIST_LEN: usize>(
    buttons: [Element<'a, Message>; LIST_LEN],
    num_items: usize,
    index: usize,
    title: Text<'a>,
    scroll_option: ScrollOption,
    cont_style: fn(&Theme) -> ContainerStyle,
) -> Container<'a, Message> {
    let mut main_col = column![title].spacing(SPACING).width(Length::Fill);

    for button in buttons {
        main_col = main_col.push(button);
    }

    let top_len;
    let bottom_len;
    let can_scroll_up;
    let can_scroll_down;

    if num_items <= LIST_LEN {
        top_len = 0;
        bottom_len = 0;
        can_scroll_up = false;
        can_scroll_down = false;
    } else {
        top_len = index as u16;
        bottom_len = (num_items - LIST_LEN - index) as u16;
        can_scroll_up = index > 0;
        can_scroll_down = index + LIST_LEN < num_items;
    }

    let top_len = match top_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let bottom_len = match bottom_len {
        0 => Length::Shrink,
        other => Length::FillPortion(other),
    };

    let scroll_btn_style =
        if cont_style(&Theme::default()).background == Some(Background::Color(blue())) {
            blue_with_border_button
        } else {
            blue_button
        };

    let mut up_btn = button(
        container(
            Svg::new(svg::Handle::from_memory(
                &include_bytes!("../../../resources/arrow_drop_up.svg")[..],
            ))
            .style(white_svg),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_container),
    )
    .width(Length::Fixed(MIN_BUTTON_SIZE))
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(scroll_btn_style);

    let mut down_btn = button(
        container(
            Svg::new(svg::Handle::from_memory(
                &include_bytes!("../../../resources/arrow_drop_down.svg")[..],
            ))
            .style(white_svg),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_container),
    )
    .width(Length::Fixed(MIN_BUTTON_SIZE))
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .style(scroll_btn_style);

    if can_scroll_up {
        up_btn = up_btn.on_press(Message::Scroll {
            which: scroll_option,
            up: true,
        });
    }

    if can_scroll_down {
        down_btn = down_btn.on_press(Message::Scroll {
            which: scroll_option,
            up: false,
        });
    }

    let scroll_bar = row![]
        .width(Length::Fill)
        .height(Length::Fill)
        .push(horizontal_space())
        .push(
            container(column![
                Space::with_height(top_len),
                container(vertical_space())
                    .width(Length::Fill)
                    .height(Length::FillPortion(LIST_LEN as u16))
                    .style(gray_container),
                Space::with_height(bottom_len),
            ])
            .padding(PADDING)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(scroll_bar_container),
        )
        .push(horizontal_space());

    container(
        row![
            main_col,
            column![up_btn, scroll_bar, down_btn]
                .spacing(SPACING)
                .width(Length::Fixed(MIN_BUTTON_SIZE))
                .height(Length::Fill),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(cont_style)
}

/// Team timeouts can be cancelled (undone) for this long after they start.
pub(in super::super) const TIMEOUT_GRACE_SECS: u16 = 15;

/// True while a team timeout is still inside its cancel/grace window.
/// `remaining` is the timeout's remaining seconds (from the snapshot);
/// `team_timeout_duration` is the configured full length.
pub(in super::super) fn team_timeout_in_grace(
    team_timeout_duration: Duration,
    remaining: u16,
) -> bool {
    (team_timeout_duration.as_secs() as u16).saturating_sub(remaining) < TIMEOUT_GRACE_SECS
}

pub(in super::super) fn build_timeout_ribbon<'a>(
    snapshot: &GameSnapshot,
    tm: &Arc<Mutex<TournamentManager>>,
    mode: Mode,
    revive_hold: Option<(GameColor, RevivePhase)>,
) -> Row<'a, Message> {
    let tm = tm.lock().unwrap();
    let black_phase = match revive_hold {
        Some((GameColor::Black, p)) => Some(p),
        _ => None,
    };
    let white_phase = match revive_hold {
        Some((GameColor::White, p)) => Some(p),
        _ => None,
    };

    let team_to_dur = tm.config().team_timeout_duration;

    let black: Element<'a, Message> = match snapshot.timeout {
        None => {
            if black_phase == Some(RevivePhase::Deciding) {
                // Revived, still held: YELLOW "release to bank / hold to start" window.
                // The mouse_area keeps the same handlers and layout slot across the
                // colour change, so the release/exit event is still captured when it
                // arrives (mouse_area holds no retained press state of its own).
                mouse_area(
                    make_multi_label_button((fl!("timeout"), fl!("revive-deciding-line-2")))
                        .style(yellow_button_armed),
                )
                .on_press(Message::TimeoutRevivePressed(GameColor::Black))
                .on_release(Message::TimeoutReviveReleased(GameColor::Black))
                .on_exit(Message::TimeoutReviveReleased(GameColor::Black))
                .into()
            } else if tm.can_revive_team_timeout(GameColor::Black).is_ok() {
                // Used-up: greyed normally; RED while in the Reviving phase. The inner
                // button has no `on_press`, so the mouse_area captures the press/hold.
                let face = if black_phase == Some(RevivePhase::Reviving) {
                    make_multi_label_button((fl!("revive-hold-line-1"), fl!("revive-hold-line-2")))
                        .style(red_button_armed)
                } else {
                    make_multi_label_button((
                        fl!("dark-timeout-line-1"),
                        fl!("dark-timeout-line-2"),
                    ))
                    .style(black_button)
                };
                mouse_area(face)
                    .on_press(Message::TimeoutRevivePressed(GameColor::Black))
                    .on_release(Message::TimeoutReviveReleased(GameColor::Black))
                    .on_exit(Message::TimeoutReviveReleased(GameColor::Black))
                    .into()
            } else {
                make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                    .on_press_maybe(
                        tm.can_start_team_timeout(GameColor::Black)
                            .ok()
                            .map(|_| Message::TeamTimeout(GameColor::Black, false)),
                    )
                    .style(black_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::Black(remaining)) => {
            if team_timeout_in_grace(team_to_dur, remaining) {
                make_multi_label_button((
                    fl!("cancel-timeout-line-1"),
                    fl!("cancel-timeout-line-2"),
                ))
                .on_press(Message::CancelTimeout)
                .style(orange_button)
                .into()
            } else {
                make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                    .on_press(Message::EndTimeout)
                    .style(red_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::White(other_remaining)) => {
            if team_timeout_in_grace(team_to_dur, other_remaining)
                && tm.can_switch_to_team_timeout(GameColor::Black).is_ok()
            {
                make_multi_label_button((fl!("switch-to"), fl!("dark-team-name-caps")))
                    .on_press(Message::TeamTimeout(GameColor::Black, true))
                    .style(black_button)
                    .into()
            } else {
                make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                    .style(black_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("dark-timeout-line-1"), fl!("dark-timeout-line-2")))
                .style(black_button)
                .into()
        }
    };

    let white: Element<'a, Message> = match snapshot.timeout {
        None => {
            if white_phase == Some(RevivePhase::Deciding) {
                mouse_area(
                    make_multi_label_button((fl!("timeout"), fl!("revive-deciding-line-2")))
                        .style(yellow_button_armed),
                )
                .on_press(Message::TimeoutRevivePressed(GameColor::White))
                .on_release(Message::TimeoutReviveReleased(GameColor::White))
                .on_exit(Message::TimeoutReviveReleased(GameColor::White))
                .into()
            } else if tm.can_revive_team_timeout(GameColor::White).is_ok() {
                let face = if white_phase == Some(RevivePhase::Reviving) {
                    make_multi_label_button((fl!("revive-hold-line-1"), fl!("revive-hold-line-2")))
                        .style(red_button_armed)
                } else {
                    make_multi_label_button((
                        fl!("light-timeout-line-1"),
                        fl!("light-timeout-line-2"),
                    ))
                    .style(white_button)
                };
                mouse_area(face)
                    .on_press(Message::TimeoutRevivePressed(GameColor::White))
                    .on_release(Message::TimeoutReviveReleased(GameColor::White))
                    .on_exit(Message::TimeoutReviveReleased(GameColor::White))
                    .into()
            } else {
                make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                    .on_press_maybe(
                        tm.can_start_team_timeout(GameColor::White)
                            .ok()
                            .map(|_| Message::TeamTimeout(GameColor::White, false)),
                    )
                    .style(white_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::White(remaining)) => {
            if team_timeout_in_grace(team_to_dur, remaining) {
                make_multi_label_button((
                    fl!("cancel-timeout-line-1"),
                    fl!("cancel-timeout-line-2"),
                ))
                .on_press(Message::CancelTimeout)
                .style(orange_button)
                .into()
            } else {
                make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2")))
                    .on_press(Message::EndTimeout)
                    .style(red_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::Black(other_remaining)) => {
            if team_timeout_in_grace(team_to_dur, other_remaining)
                && tm.can_switch_to_team_timeout(GameColor::White).is_ok()
            {
                make_multi_label_button((fl!("switch-to"), fl!("light-team-name-caps")))
                    .on_press(Message::TeamTimeout(GameColor::White, true))
                    .style(white_button)
                    .into()
            } else {
                make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                    .style(white_button)
                    .into()
            }
        }
        Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("light-timeout-line-1"), fl!("light-timeout-line-2")))
                .style(white_button)
                .into()
        }
    };

    let referee = match snapshot.timeout {
        None => make_multi_label_button((fl!("ref-timeout-line-1"), fl!("ref-timeout-line-2")))
            .on_press_maybe(
                tm.can_start_ref_timeout()
                    .ok()
                    .map(|_| Message::RefTimeout(false)),
            )
            .style(yellow_button),
        Some(TimeoutSnapshot::Ref(_)) => make_multi_label_button((
            fl!("cancel-ref-timeout-line-1"),
            fl!("cancel-ref-timeout-line-2"),
        ))
        .on_press(Message::EndTimeout)
        .style(orange_button),
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => match tm.can_switch_to_ref_timeout() {
            Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("ref")))
                .on_press(Message::RefTimeout(true))
                .style(yellow_button),
            Err(_) => {
                make_multi_label_button((fl!("ref-timeout-line-1"), fl!("ref-timeout-line-2")))
                    .style(yellow_button)
            }
        },
    };

    let penalty = match snapshot.timeout {
        None => make_multi_label_button((fl!("penalty-shot-line-1"), fl!("penalty-shot-line-2")))
            .on_press_maybe(
                tm.can_start_penalty_shot()
                    .ok()
                    .map(|_| Message::PenaltyShot(false)),
            )
            .style(red_button),
        Some(TimeoutSnapshot::PenaltyShot(_)) => {
            make_multi_label_button((fl!("cancel-pen-shot-line-1"), fl!("cancel-pen-shot-line-2")))
                .on_press(Message::EndTimeout)
                .style(orange_button)
        }
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_)) => {
            let can_switch = if mode == Mode::Rugby {
                tm.can_switch_to_rugby_penalty_shot()
            } else {
                tm.can_switch_to_penalty_shot()
            };
            match can_switch {
                Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("pen-shot")))
                    .on_press(Message::PenaltyShot(true))
                    .style(red_button),
                Err(_) => make_multi_label_button((
                    fl!("penalty-shot-line-1"),
                    fl!("penalty-shot-line-2"),
                ))
                .style(red_button),
            }
        }
    };

    drop(tm);

    row![black, referee, penalty, white].spacing(SPACING)
}

/// Build the portal-health tile shown at the left end of the time banner.
///
/// The tile is a `tile_size x tile_size` square. Callers pass the
/// banner's outer height so the tile fills the banner vertically on
/// both "tall" and "short" pages; the status dot scales proportionally
/// from `HEALTH_DOT_SIZE / HEALTH_TILE_SIZE`.
///
/// The UWH Portal logo sits above the coloured dot. The dot's colour
/// reflects `state.health` (Green / Yellow / Red). The whole tile is
/// a button that fires `Message::OpenPortalDetailPage` when tapped.
/// Returns the operator-facing sport prefix for portal strings.
/// "UWH" for underwater hockey modes; "UWR" for underwater rugby.
/// View builders pass this into fl!() for keys that say `{ $portal }`.
pub(crate) fn portal_name_for_mode(mode: Mode) -> &'static str {
    match mode {
        Mode::Rugby => "UWR",
        Mode::Hockey6V6 | Mode::Hockey3V3 => "UWH",
        Mode::BeepTest => "",
    }
}

pub(crate) fn crosses_portal(old: Mode, new: Mode) -> bool {
    portal_name_for_mode(old) != portal_name_for_mode(new)
}

pub(super) fn make_health_tile<'a>(
    state: PortalIndicatorState,
    tile_size: f32,
    mode: Mode,
) -> Element<'a, Message> {
    let dot_size = tile_size * HEALTH_DOT_SIZE / HEALTH_TILE_SIZE;

    let dot_color = match state.health {
        HealthState::Green => green(),
        HealthState::Yellow => yellow(),
        HealthState::Red => red(),
    };

    let dot_style = move |_theme: &Theme| ContainerStyle {
        background: Some(Background::Color(dot_color)),
        text_color: None,
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: Radius::new(dot_size / 2.0),
        },
        shadow: Default::default(),
    };

    let dot = container(Space::new(Length::Fill, Length::Fill))
        .width(Length::Fixed(dot_size))
        .height(Length::Fixed(dot_size))
        .style(dot_style);

    // Logo picks the sport's portal emblem: UWR Compact Logo in Rugby
    // mode, otherwise the UWH Portal Compact Logo. See ADR 016 for the
    // broader UWR mode portal-routing work (pre-existing issue where
    // the URL itself is not mode-aware; this file handles the visual
    // side only).
    let logo_bytes: &[u8] = match mode {
        Mode::Rugby => &include_bytes!("../../../resources/UWR_Compact_Logo.png")[..],
        Mode::Hockey6V6 | Mode::Hockey3V3 => {
            &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..]
        }
        Mode::BeepTest => &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..],
    };
    let logo = Image::new(image::Handle::from_bytes(logo_bytes))
        .width(Length::Fill)
        .height(Length::Fill);

    let tile_contents = column![
        container(logo)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        container(dot).width(Length::Fill).center_x(Length::Fill),
    ]
    .spacing(SPACING / 2.0)
    .align_x(Alignment::Center)
    .width(Length::Fill);

    button(
        container(tile_contents)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(Length::Fixed(tile_size))
    .height(Length::Fixed(tile_size))
    .padding(0)
    .style(light_gray_button)
    .on_press(Message::OpenPortalDetailPage)
    .into()
}

pub(super) fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    tall: bool,
    editing_time: bool,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
    overrun_label: Option<String>,
) -> Row<'a, Message> {
    let make_red = if editing_time {
        false
    } else {
        match snapshot.timeout {
            Some(TimeoutSnapshot::Black(time)) | Some(TimeoutSnapshot::White(time)) => {
                (time <= 10 && (time % 2 == 0) && (time != 0)) || time == 15
            }
            Some(TimeoutSnapshot::Ref(_)) | Some(TimeoutSnapshot::PenaltyShot(_)) => false,
            None => {
                let is_alert_period = match snapshot.current_period {
                    GamePeriod::BetweenGames
                    | GamePeriod::HalfTime
                    | GamePeriod::PreOvertime
                    | GamePeriod::OvertimeHalfTime
                    | GamePeriod::PreSuddenDeath => true,
                    GamePeriod::FirstHalf
                    | GamePeriod::SecondHalf
                    | GamePeriod::OvertimeFirstHalf
                    | GamePeriod::OvertimeSecondHalf
                    | GamePeriod::SuddenDeath => false,
                };

                snapshot.current_period != GamePeriod::SuddenDeath
                    && ((snapshot.secs_in_period <= 10
                        && (snapshot.secs_in_period % 2 == 0)
                        && (snapshot.secs_in_period != 0))
                        || (is_alert_period && snapshot.secs_in_period == 30))
            }
        }
    };

    let (mut period_text, period_color): (_, fn(&Theme) -> TextStyle) = {
        let (text, color): (_, fn(&Theme) -> TextStyle) = match snapshot.current_period {
            GamePeriod::BetweenGames => (fl!("next-game"), yellow_text),
            GamePeriod::FirstHalf => (fl!("first-half"), green_text),
            GamePeriod::HalfTime => (fl!("half-time"), yellow_text),
            GamePeriod::SecondHalf => (fl!("second-half"), green_text),
            GamePeriod::PreOvertime => (fl!("pre-ot-break-full"), yellow_text),
            GamePeriod::OvertimeFirstHalf => (fl!("overtime-first-half"), green_text),
            GamePeriod::OvertimeHalfTime => (fl!("overtime-half-time"), yellow_text),
            GamePeriod::OvertimeSecondHalf => (fl!("overtime-second-half"), green_text),
            GamePeriod::PreSuddenDeath => (fl!("pre-sudden-death-break"), yellow_text),
            GamePeriod::SuddenDeath => (fl!("sudden-death"), green_text),
        };

        if make_red {
            (text, black_text)
        } else {
            (text, color)
        }
    };

    if tall && (snapshot.timeout.is_some()) {
        match snapshot.current_period {
            GamePeriod::PreOvertime => period_text = fl!("pre-ot-break-abreviated"),
            GamePeriod::OvertimeFirstHalf => period_text = fl!("ot-first-half"),
            GamePeriod::OvertimeHalfTime => period_text = fl!("ot-half-time"),
            GamePeriod::OvertimeSecondHalf => period_text = fl!("ot-2nd-half"),
            GamePeriod::PreSuddenDeath => period_text = fl!("pre-sd-break"),
            _ => {}
        };
    }

    macro_rules! make_time_view {
        ($base:ident, $per_text:ident, $time_text:ident) => {
            $base.width(Length::Fill).push($per_text).push($time_text)
        };
    }

    let make_time_view_row = |period_text, time_text, style: fn(&Theme) -> TextStyle| {
        // Wrap period text in a right-aligned container so the text widget uses
        // width(Shrink). This ensures iced's damage region starts from the text's
        // actual left edge, preventing old glyph pixels from bleeding through when
        // the period name changes (iced 0.13 damage tracking bug with aligned text).
        let per = container(
            text(period_text)
                .style(style)
                .width(Length::Shrink)
                .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Center);
        let time = text(time_text)
            .style(style)
            .size(LARGE_TEXT)
            .width(Length::Fill)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Left);
        let r = row![].spacing(SPACING);
        make_time_view!(r, per, time).align_y(Alignment::Center)
    };

    // The banner is "tight" only in UWR + portal mode (both side tiles present)
    // AND when a second middle column is also competing for width -- either a
    // timeout column or the delay figure. In that case the period label and clock
    // shrink so everything fits; every other banner keeps the big full-size clock
    // for poolside readability.
    let compact = portal_indicator.is_some()
        && mode == Mode::Rugby
        && (snapshot.timeout.is_some() || overrun_label.is_some());

    let make_time_view_col = |period_text, time_text, style| {
        let per = if compact {
            text(period_text).style(style).size(SMALL_TEXT)
        } else {
            text(period_text).style(style)
        };
        let time =
            text(time_text)
                .style(style)
                .size(if compact { MEDIUM_TEXT } else { LARGE_TEXT });
        let c = column![];
        make_time_view!(c, per, time).align_x(Alignment::Center)
    };

    let mut content = row![]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_y(Alignment::Center);

    let timeout_info = snapshot.timeout.map(|t| -> (_, fn(&Theme) -> TextStyle) {
        match t {
            TimeoutSnapshot::White(_) => (
                if tall {
                    fl!("white-timeout-short")
                } else {
                    fl!("white-timeout-full")
                },
                if make_red { black_text } else { white_text },
            ),
            TimeoutSnapshot::Black(_) => (
                if tall {
                    fl!("black-timeout-short")
                } else {
                    fl!("black-timeout-full")
                },
                black_text,
            ),
            TimeoutSnapshot::Ref(_) => (fl!("ref-timeout-short"), yellow_text),
            TimeoutSnapshot::PenaltyShot(_) => (fl!("penalty-shot-short"), red_text),
        }
    });

    let time_text = secs_to_long_time_string(snapshot.secs_in_period);

    let time_text = time_text.trim().to_owned();

    if tall {
        content = content.push(make_time_view_col(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_col(
                timeout_text,
                timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    } else {
        content = content.push(make_time_view_row(period_text, time_text, period_color));
        if let Some((timeout_text, timeout_color)) = timeout_info {
            content = content.push(make_time_view_row(
                timeout_text,
                timeout_time_string(snapshot),
                timeout_color,
            ));
        }
    }

    // The delay figure yields its slot to an active timeout: the banner cannot
    // hold the period/clock, a timeout column, the delay, the portal tile, and the
    // UWR pause button at once. During a timeout the delay is hidden (it keeps
    // accruing) and reappears, updated, once the timeout ends.
    if let Some(label) = overrun_label {
        if snapshot.timeout.is_none() {
            // Build the DELAY block with the same helper as the period/clock so the
            // label and figure match the game time's size and vertical alignment
            // exactly (it tracks `compact` the same way).
            content = content.push(make_time_view_col(fl!("delay"), label, red_text));
        }
    }

    // The tile fills the banner height so it looks visually balanced on
    // both tall (Main page, with a "NEXT GAME" label above the clock)
    // and short banners. On short banners the tile is MIN_BUTTON_SIZE
    // square; on tall banners it grows to match the taller banner.
    let tile_size = if tall {
        HEALTH_TILE_SIZE + PADDING + SMALL_PLUS_TEXT
    } else {
        HEALTH_TILE_SIZE
    };
    let button_height = Length::Fixed(tile_size);

    let button_style = if make_red { red_button } else { gray_button };

    let time_button = button(content)
        .width(Length::Fill)
        .height(button_height)
        .style(button_style)
        .padding(PADDING)
        .on_press(if editing_time {
            Message::NoAction
        } else {
            Message::EditTime
        });

    // When no portal event is linked, the health tile is not rendered
    // and the banner falls back to the pre-feature layout. See
    // `ViewData.portal_indicator` and ADR 011 amendment 2026-04-23.
    let mut time_row = if let Some(state) = portal_indicator {
        row![make_health_tile(state, tile_size, mode), time_button]
    } else {
        row![time_button]
    }
    .height(button_height)
    .width(Length::Fill)
    .spacing(SPACING)
    .align_y(Alignment::Center);

    if mode == Mode::Rugby {
        let play_pause_icon = container(
            Svg::new(svg::Handle::from_memory(if clock_running {
                &include_bytes!("../../../resources/pause.svg")[..]
            } else {
                &include_bytes!("../../../resources/play_arrow.svg")[..]
            }))
            .style(black_svg)
            .height(Length::Fixed(LARGE_TEXT * 1.2)),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(transparent_container);
        let mut play_pause_button = button(play_pause_icon)
            .style(gray_button)
            .height(button_height)
            .width(Length::Fixed(MIN_BUTTON_SIZE));
        if !editing_time {
            play_pause_button = play_pause_button.on_press(if clock_running {
                Message::StopClock
            } else {
                Message::StartClock
            });
        };
        time_row = time_row.push(play_pause_button);
    };

    time_row
}

pub(super) fn make_time_editor<'a, T: IntoFragment<'a>>(
    title: T,
    time: Duration,
    timeout: bool,
    value_color: Option<iced::Color>,
) -> Container<'a, Message> {
    let wide = time > Duration::from_secs(MAX_STRINGABLE_SECS as u64);

    let min_edits = column![
        make_small_button("+", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: true,
                secs: 60,
                timeout,
            }),
        make_small_button("-", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: false,
                secs: 60,
                timeout,
            }),
    ]
    .spacing(SPACING);

    let sec_edits = column![
        make_small_button("+", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: true,
                secs: 1,
                timeout,
            }),
        make_small_button("-", LARGE_TEXT)
            .style(blue_button)
            .on_press(Message::ChangeTime {
                increase: false,
                secs: 1,
                timeout,
            }),
    ]
    .spacing(SPACING);

    let time_col = column![
        {
            let t = text(time_string(time)).size(LARGE_TEXT);
            match value_color {
                Some(c) => t.color(c),
                None => t,
            }
        },
        row![
            horizontal_space(),
            make_small_button(fl!("zero"), MEDIUM_TEXT)
                .style(blue_button)
                .on_press(Message::ChangeTime {
                    increase: false,
                    secs: u64::MAX,
                    timeout,
                }),
            horizontal_space(),
        ],
    ]
    .align_x(Horizontal::Center)
    .width(Length::Fixed(if wide { 300.0 } else { 200.0 }))
    .spacing(SPACING);

    let time_edit = row![min_edits, time_col, sec_edits]
        .spacing(SPACING)
        .align_y(Alignment::Center);

    container(
        column![text(title).size(MEDIUM_TEXT), time_edit]
            .spacing(SPACING)
            .align_x(Alignment::Center),
    )
    .style(light_gray_container)
    .padding(PADDING)
}

pub(super) fn time_string(time: Duration) -> String {
    secs_to_long_time_string(time.as_secs()).trim().to_string()
}

pub(super) fn timeout_time_string(snapshot: &GameSnapshot) -> String {
    match snapshot.timeout {
        Some(TimeoutSnapshot::Black(secs))
        | Some(TimeoutSnapshot::White(secs))
        | Some(TimeoutSnapshot::Ref(secs))
        | Some(TimeoutSnapshot::PenaltyShot(secs)) => secs_to_time_string(secs).trim().to_string(),
        None => String::new(),
    }
}

pub(super) fn bool_string(val: bool) -> String {
    match val {
        true => fl!("yes"),
        false => fl!("no"),
    }
}

pub(super) fn penalty_string(penalties: &[PenaltySnapshot]) -> String {
    let mut string = String::new();

    for pen in penalties.iter() {
        write!(&mut string, "#{} - ", pen.player_number).unwrap();
        match pen.time {
            PenaltyTime::Seconds(secs) => {
                if secs != 0 {
                    writeln!(&mut string, "{}:{:02}", secs / 60, secs % 60).unwrap();
                } else {
                    string += &fl!("served");
                    string += "\n";
                }
            }
            PenaltyTime::TotalDismissal => {
                string += &fl!("total-dismissal");
                string += "\n"
            }
        }
    }
    // if the string is not empty, the last char is a '\n' that we don't want
    string.pop();
    string
}

pub(super) fn game_string_long(game: &Game, teams: Option<&TeamList>, len_limit: usize) -> String {
    let black = get_team_name(&game.dark, teams);
    let white = get_team_name(&game.light, teams);

    let black = limit_team_name_len(&black, len_limit);
    let white = limit_team_name_len(&white, len_limit);

    format!("{} - {} vs {}", game.number, black, white)
}

pub(super) fn get_team_name(team: &ScheduledTeam, teams: Option<&TeamList>) -> String {
    if let (Some(id), Some(teams)) = (team.assigned(), teams) {
        teams
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.full().to_string())
    } else if let Some(result_of) = team.result_of() {
        match result_of {
            ResultOf::Loser { game_number } => format!("L_{game_number}"),
            ResultOf::Winner { game_number } => format!("W_{game_number}"),
        }
    } else if let Some(seed) = team.seeded_by() {
        let group = seed.group.as_deref().unwrap_or("Unknown");
        format!("Seed {} of {}", seed.number, group)
    } else if let Some(s) = team.pending() {
        s.to_string()
    } else {
        "Unknown".to_string()
    }
}

pub(super) fn limit_team_name_len(name: &str, len_limit: usize) -> String {
    const ELIPSIS: [char; 3] = ['.', '.', '.'];

    if name.len() > len_limit {
        name.chars().take(len_limit - 1).chain(ELIPSIS).collect()
    } else {
        name.to_owned()
    }
}

pub(super) fn config_string_game_num(
    snapshot: &GameSnapshot,
    using_uwhportal: bool,
    games: Option<&GameList>,
) -> (String, GameNumber) {
    let mut result = String::new();
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if using_uwhportal {
            if let Some(games) = games {
                prev_game = match games.get(&snapshot.game_number) {
                    Some(game) => game.number.to_string(),
                    None if snapshot.game_number == "0" => fl!("none"),
                    None => fl!("error", number = snapshot.game_number.clone()),
                };
                next_game = match games.get(&snapshot.next_game_number) {
                    Some(game) => game.number.to_string(),
                    None => fl!("error", number = snapshot.next_game_number.clone()),
                };
            } else {
                prev_game = if snapshot.game_number == "0" {
                    fl!("none")
                } else {
                    fl!("error", number = snapshot.game_number.clone())
                };
                next_game = fl!("error", number = snapshot.next_game_number.clone());
            }
        } else {
            prev_game = if snapshot.game_number == "0" {
                fl!("none")
            } else {
                snapshot.game_number.to_string()
            };
            next_game = snapshot.next_game_number.to_string();
        }

        result += &fl!("two-games", prev_game = prev_game, next_game = next_game);
        result += "\n\n";
        snapshot.next_game_number.clone()
    } else {
        let game;
        if using_uwhportal {
            if let Some(games) = games {
                game = match games.get(&snapshot.game_number) {
                    Some(game) => game.number.to_string(),
                    None => fl!("error", number = snapshot.game_number.clone()),
                };
            } else {
                game = fl!("error", number = snapshot.game_number.clone());
            }
        } else {
            game = snapshot.game_number.to_string();
        }
        result += &fl!("one-game", game = game);
        result += "\n\n";
        snapshot.game_number.clone()
    };

    (result, game_number)
}

pub(super) fn config_string(
    snapshot: &GameSnapshot,
    config: &GameConfig,
    using_uwhportal: bool,
    schedule: Option<&Schedule>,
    teams: Option<&TeamList>,
) -> String {
    const TEAM_NAME_LEN_LIMIT: usize = 40;
    let games = schedule.map(|s| &s.games);
    let (result_string, _) = config_string_game_num(snapshot, using_uwhportal, games);
    let mut result = result_string;
    let (_, result_u32) = config_string_game_num(snapshot, using_uwhportal, games);
    let game_number = result_u32;

    if using_uwhportal {
        if let Some(games) = games {
            if let Some(game) = games.get(&game_number) {
                let black = get_team_name(&game.dark, teams);
                let white = get_team_name(&game.light, teams);
                result += &fl!(
                    "teams",
                    dark_team = limit_team_name_len(&black, TEAM_NAME_LEN_LIMIT),
                    light_team = limit_team_name_len(&white, TEAM_NAME_LEN_LIMIT)
                );
                result += "\n";
            }
        }
    }

    // Game Block (the start-to-start slot) sits right after the game number and
    // before the play-length lines.
    result += &fl!(
        "game-block-info",
        game_block = time_string(config.game_block)
    );
    result += "\n";

    result += &if config.single_half {
        // Single-period game: show "Game Length" and omit the half-time line.
        fl!(
            "game-config-single-half",
            half_len = time_string(config.half_play_duration),
            sd_allowed = bool_string(config.sudden_death_allowed),
            ot_allowed = bool_string(config.overtime_allowed)
        )
    } else {
        fl!(
            "game-config",
            half_len = time_string(config.half_play_duration),
            half_time_len = time_string(config.half_time_duration),
            sd_allowed = bool_string(config.sudden_death_allowed),
            ot_allowed = bool_string(config.overtime_allowed)
        )
    };

    let team_timeouts_value = if config.num_team_timeouts_allowed == 0 {
        "0".to_string()
    } else if config.timeouts_counted_per_half {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
    } else {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
    };
    result += "\n";
    result += &fl!("team-timeouts", value = team_timeouts_value);

    let stop_clock = if let Some(sched) = schedule {
        if let Some(timing_rule) = sched.get_game_timing(&game_number) {
            bool_string(timing_rule.last_2_min_stop_time)
        } else {
            fl!("unknown")
        }
    } else {
        fl!("unknown")
    };
    result += "\n";
    result += &fl!("stop-clock-last-2", stop_clock = stop_clock);
    result += "\n";

    let mut chief_ref = "-".to_string();
    let mut timer = "-".to_string();
    let mut water_ref_1 = "-".to_string();
    let mut water_ref_2 = "-".to_string();
    let mut water_ref_3 = "-".to_string();

    if let Some(games) = games {
        if let Some(game) = games.get(&game_number) {
            if let Some(refs) = &game.referee_assignments {
                for ref_assignment in refs {
                    if ref_assignment.user_id.is_some() {
                        // Fall back to '-' for unassigned slots — language-neutral,
                        // visually distinct from real names.
                        let display = ref_assignment
                            .display_name
                            .clone()
                            .unwrap_or_else(|| "-".to_string());
                        match ref_assignment.role.as_str() {
                            "Chief" => chief_ref = display,
                            "TimeOrScoreKeeper" => timer = display,
                            "Water1" => water_ref_1 = display,
                            "Water2" => water_ref_2 = display,
                            "Water3" => water_ref_3 = display,
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    result += &fl!(
        "ref-list",
        chief_ref = chief_ref,
        timer = timer,
        water_ref_1 = water_ref_1,
        water_ref_2 = water_ref_2,
        water_ref_3 = water_ref_3
    );

    result
}

pub(super) fn make_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    let t = text(label).width(Length::Shrink);
    button(container(t).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_smaller_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    let t = text(label).width(Length::Shrink);
    button(container(t).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(XS_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_multi_label_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    labels: (T, T),
) -> Button<'a, Message> {
    let t0 = text(labels.0)
        .align_x(Horizontal::Left)
        .width(Length::Shrink);
    let t1 = text(labels.1)
        .align_x(Horizontal::Left)
        .width(Length::Shrink);
    button(
        container(
            column![
                container(t0).center_x(Length::Fill),
                container(t1).center_x(Length::Fill),
            ]
            .width(Length::Fill),
        )
        .center(Length::Fill),
    )
    .padding(PADDING)
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .width(Length::Fill)
}

pub(super) enum NameLines<T> {
    /// Name at the app-default text size. Used for short names like "TÜRKÇE".
    OneLine(T),
    /// Name at SMALL_TEXT. Used for long names like "BAHASA INDONESIA" that don't
    /// comfortably fit at the default size alongside the UNVERIFIED note below them.
    OneLineSmall(T),
}

pub(super) fn make_lang_button_with_note<'a, Message, T>(
    main: NameLines<T>,
    note: T,
    font: Option<iced_core::Font>,
) -> Button<'a, Message>
where
    Message: 'a + Clone,
    T: IntoFragment<'a>,
{
    let with_font = |t: Text<'a>| -> Text<'a> { if let Some(f) = font { t.font(f) } else { t } };
    let note_text = with_font(
        text(note)
            .size(SMALL_TEXT)
            .align_x(Horizontal::Left)
            .width(Length::Shrink),
    );
    let name_text = match main {
        NameLines::OneLine(name) => {
            with_font(text(name).align_x(Horizontal::Left).width(Length::Shrink))
        }
        NameLines::OneLineSmall(name) => with_font(
            text(name)
                .size(SMALL_TEXT)
                .align_x(Horizontal::Left)
                .width(Length::Shrink),
        ),
    };
    let name_column = column![
        container(name_text).center_x(Length::Fill),
        container(note_text).center_x(Length::Fill),
    ];
    button(container(name_column.width(Length::Fill)).center(Length::Fill))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

pub fn centered_text<'a, T: IntoFragment<'a>>(label: T) -> Text<'a> {
    text(label)
        .align_y(Vertical::Center)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

pub(super) fn make_small_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
    size: f32,
) -> Button<'a, Message> {
    let t = text(label)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Center)
        .width(Length::Shrink)
        .size(size);
    button(container(t).center(Length::Fill))
        .width(Length::Fixed(MIN_BUTTON_SIZE))
        .height(Length::Fixed(MIN_BUTTON_SIZE))
}

pub(super) fn make_value_button<'a, T, U>(
    first_label: T,
    second_label: U,
    large_text: (bool, bool),
    message: Option<Message>,
) -> Button<'a, Message>
where
    Message: 'a + Clone,
    T: IntoFragment<'a>,
    U: IntoFragment<'a>,
{
    let mut button = button(
        row![
            text(first_label)
                .size(if large_text.0 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .height(Length::Fill)
                .align_y(Vertical::Center),
            horizontal_space(),
            text(second_label)
                .size(if large_text.1 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .height(Length::Fill)
                .align_y(Vertical::Center),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center)
        .padding(PADDING),
    )
    .height(Length::Fixed(MIN_BUTTON_SIZE))
    .width(Length::Fill)
    .style(light_gray_button);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

pub(super) fn make_penalty_dropdown<'a>(
    infraction: Infraction,
    display_infraction_name: bool,
) -> Element<'a, Message> {
    const ROW_LEN: usize = 6;
    let foul_buttons = all::<Infraction>().map(|button_infraction| {
        // The Unknown infraction's icon is a black "?" PNG, invisible on the
        // black High-Contrast tile. In High Contrast only, render a themed white
        // "?" instead so the Unknown option follows the display mode; Light and
        // Dark keep the original image.
        let inner: Element<'a, Message> = if button_infraction == Infraction::Unknown
            && display_mode() == DisplayMode::HighContrast
        {
            container(text("?").size(LARGE_TEXT).style(white_text))
                .center(Length::Fill)
                .style(transparent_container)
                .into()
        } else {
            container(
                Image::new(image::Handle::from_bytes(button_infraction.get_image()))
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE)),
            )
            .style(transparent_container)
            .into()
        };
        button(inner)
            .padding(0)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .width(Length::Fill)
            .style(if infraction == button_infraction {
                light_gray_selected_button
            } else {
                light_gray_button
            })
            .on_press(Message::ChangeInfraction(button_infraction))
    });

    let name: Container<'_, Message> = container(
        row![text(fl!(
            "infraction",
            infraction = inf_short_name(infraction)
        ))]
        .spacing(0)
        .align_y(Alignment::Center),
    )
    .style(blue_container)
    .width(Length::Fill);

    let mut first_row = row![].spacing(SPACING);
    for button in foul_buttons.clone().take(ROW_LEN) {
        first_row = first_row.push(button);
    }
    let mut second_row = row![].spacing(SPACING);
    for button in foul_buttons.skip(ROW_LEN).take(ROW_LEN) {
        second_row = second_row.push(button);
    }

    let open_button_content = if display_infraction_name {
        column![
            name,
            Space::with_height(SPACING),
            first_row,
            Space::with_height(SPACING),
            second_row,
        ]
        .padding(0)
    } else {
        column![first_row, Space::with_height(SPACING), second_row,].padding(0)
    };

    container(open_button_content)
        .padding(PADDING)
        .width(Length::Fill)
        .style(blue_container)
        .into()
}

pub fn make_warning_container<'a>(
    warning: &InfractionSnapshot,
    color: Option<GameColor>,
) -> Container<'a, Message> {
    const WIDTH: u16 = 220;
    const HEIGHT: u16 = 23;

    let who = if let Some(num) = warning.player_number {
        format!("#{num}")
    } else {
        fl!("team-warning-abreviation")
    };

    container(if color.is_some() {
        row![
            Space::with_width(PADDING),
            text(inf_short_name(warning.infraction)).size(SMALL_TEXT),
            horizontal_space(),
            text(who).size(SMALL_TEXT),
            Space::with_width(PADDING),
        ]
    } else {
        row![
            horizontal_space(),
            text(inf_short_name(warning.infraction)).size(SMALL_TEXT),
            horizontal_space(),
        ]
    })
    .width(WIDTH)
    .height(HEIGHT)
    .style(match color {
        Some(GameColor::Black) => black_container,
        Some(GameColor::White) => white_container,
        None => blue_container,
    })
    .padding(0)
}

pub fn inf_short_name(inf: Infraction) -> String {
    match inf {
        Infraction::Unknown => fl!("unknown"),
        Infraction::StickInfringement => fl!("stick-foul"),
        Infraction::IllegalAdvancement => fl!("illegal-advance"),
        Infraction::IllegalSubstitution => fl!("sub-foul"),
        Infraction::IllegallyStoppingThePuck => fl!("illegal-stoppage"),
        Infraction::OutOfBounds => fl!("out-of-bounds"),
        Infraction::GrabbingTheBarrier => fl!("grabbing-the-wall"),
        Infraction::Obstruction => fl!("obstruction"),
        Infraction::DelayOfGame => fl!("delay-of-game"),
        Infraction::UnsportsmanlikeConduct => fl!("unsportsmanlike"),
        Infraction::FreeArm => fl!("free-arm"),
        Infraction::FalseStart => fl!("false-start"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mode;

    #[test]
    fn crosses_portal_within_hockey_is_false() {
        assert!(!crosses_portal(Mode::Hockey6V6, Mode::Hockey3V3));
        assert!(!crosses_portal(Mode::Hockey3V3, Mode::Hockey6V6));
        assert!(!crosses_portal(Mode::Hockey6V6, Mode::Hockey6V6));
        assert!(!crosses_portal(Mode::Hockey3V3, Mode::Hockey3V3));
        assert!(!crosses_portal(Mode::Rugby, Mode::Rugby));
    }

    #[test]
    fn crosses_portal_hockey_to_rugby_is_true() {
        assert!(crosses_portal(Mode::Hockey6V6, Mode::Rugby));
        assert!(crosses_portal(Mode::Hockey3V3, Mode::Rugby));
    }

    #[test]
    fn crosses_portal_rugby_to_hockey_is_true() {
        assert!(crosses_portal(Mode::Rugby, Mode::Hockey6V6));
        assert!(crosses_portal(Mode::Rugby, Mode::Hockey3V3));
    }
}
