use super::fit_text::{fit_text, fit_text_lines};
use super::*;
use crate::app::RevivePhase;
use crate::portal_manager::{HealthState, PortalIndicatorState};
use crate::tournament_manager::SharedGame;
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
use std::{fmt::Write, time::Duration};
use uwh_common::{
    color::Color as GameColor,
    game_snapshot::{
        GamePeriod, GameSnapshot, Infraction, InfractionSnapshot, PenaltySnapshot, PenaltyTime,
        TimeoutSnapshot,
    },
    uwhportal::schedule::{Game, GameList, ResultOf, ScheduledTeam, TeamList},
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

    // A remembered scroll index can outlive the list it points into: e.g. the
    // portal detail list shrinks as queued games upload while the operator is
    // scrolled down. Clamp it to the last valid offset so the
    // `num_items - LIST_LEN - index` math below cannot underflow (a debug panic
    // / release u16 wrap).
    let index = index.min(num_items.saturating_sub(LIST_LEN));

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
    tm: &SharedGame,
    mode: Mode,
    revive_hold: Option<(GameColor, RevivePhase)>,
) -> Row<'a, Message> {
    let tm = tm.lock();
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
            if black_phase == Some(RevivePhase::Restored) {
                // Revived, still held: YELLOW "RESTORED" state, shown until release
                // (which confirms the restore). The mouse_area keeps the same handlers
                // and layout slot across the colour change, so the release/exit event is
                // still captured when it arrives (mouse_area holds no retained press
                // state of its own).
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
            if white_phase == Some(RevivePhase::Restored) {
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

/// A game source's display name, as the source buttons on Game Options show it.
///
/// One function so that a message naming a source and the button that chooses it
/// cannot drift apart — the confirmation's sentence has to read as the name of
/// the button the operator just pressed.
pub(crate) fn source_display_name(source: GameSource, mode: Mode) -> String {
    match source {
        GameSource::Portal => fl!("source-portal", portal = portal_name_for_mode(mode)),
        GameSource::Custom => fl!("source-custom"),
        // No source-switch target is ever Manual: MANUAL GAMES is a separate
        // control with its own label, which this returns for completeness.
        GameSource::Manual => fl!("manual-games"),
    }
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
    // A third-party site gets a generic globe: showing the official Portal's
    // emblem above a connection to somebody else's server would be a false
    // statement in the one place an operator looks to see what they are
    // connected to. Blue is taken from the palette, so it follows the display
    // mode like every other themed colour.
    let emblem: Element<'a, Message> = if state.site_is_custom {
        Svg::new(svg::Handle::from_memory(
            &include_bytes!("../../../resources/globe.svg")[..],
        ))
        .style(blue_svg)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        let logo_bytes: &[u8] = match mode {
            Mode::Rugby => &include_bytes!("../../../resources/UWR_Compact_Logo.png")[..],
            Mode::Hockey6V6 | Mode::Hockey3V3 => {
                &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..]
            }
            Mode::BeepTest => &include_bytes!("../../../resources/UWH_Portal_Compact_Logo.png")[..],
        };
        Image::new(image::Handle::from_bytes(logo_bytes))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    let tile_contents = column![
        container(emblem)
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

/// Floor for the banner's own text, below the `MIN_FIT_TEXT` that suits buttons.
///
/// The period label and the clock share a box with no spare height, so the label
/// must shrink rather than wrap. In the tightest configuration -- UWR with both
/// side tiles and a timeout, leaving the column about 103px wide -- a long
/// German period name needs to come down this far to stay on one line. A small
/// label is a much better outcome than a missing clock.
const BANNER_MIN_TEXT: f32 = 14.0;

/// Largest time size to use when two readouts share the banner.
///
/// Derived from the vertical budget rather than picked by eye. The banner button
/// leaves 126 - 2*PADDING = 110px inside. A label wrapped onto two lines at
/// `SMALL_TEXT` takes 2 * 19 * 1.3 = 49.4px of that, leaving 60.6px, which allows
/// a time of 60.6 / 1.3 = 46px. Larger than that and a wrapped label would push
/// the time out of the banner -- which is exactly how the clock used to vanish.
///
/// The times are fitted rather than pinned to this, because the two modes do not
/// give the banner the same width: UWR carries a play/pause button as well as the
/// portal tile, so its halves are narrower. Pinning one size would render UWH at
/// the size UWR needs.
const BANNER_TWO_TIME_TEXT: f32 = 46.0;

/// Shown in place of the clock when the selected court's schedule is finished. Not a
/// translated string -- it is punctuation, the same "nothing here" signal the game info
/// table draws with a hyphen.
const NO_TIME: &str = "--:--";

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

    // A blank next-game number between games means the selected court has no further
    // games (uwh-common's GameSnapshot::next_game_number reports None for it). The
    // banner says so rather than promising a game that is not coming: the label reads
    // END and the time reads dashes, keeping the banner's two-readout shape so nothing
    // around it changes size.
    let schedule_ended =
        snapshot.current_period == GamePeriod::BetweenGames && snapshot.next_game_number.is_empty();

    let (mut period_text, period_color): (_, fn(&Theme) -> TextStyle) = {
        let (text, color): (_, fn(&Theme) -> TextStyle) = match snapshot.current_period {
            GamePeriod::BetweenGames => {
                if schedule_ended {
                    (fl!("schedule-end"), yellow_text)
                } else {
                    (fl!("next-game"), yellow_text)
                }
            }
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

    // Neither banner text may wrap. The period label sits above (or beside) the
    // clock in a box with no spare height, so a second line pushes the clock out
    // of the banner and it is not drawn at all -- the clock simply vanishes. They
    // shrink to fit instead.
    //
    // This replaces a hand-tuned `compact` rule that dropped both to fixed
    // smaller sizes. It fired only in UWR + portal mode *and* during a timeout,
    // so every other crowded configuration lost the clock: Hockey with a timeout,
    // UWR without the portal tile, and UWR with both side tiles even with no
    // timeout at all. Crowding does not depend on the mode or on which side tiles
    // happen to be present, so nothing is triggered on state any more.
    //
    // `FitText` anchors its paragraphs top-left and places each line itself,
    // which is also why the right-aligned `container` workaround this row used to
    // need (for the iced 0.13 repaint bug with aligned text) is gone.
    // How a readout is sized depends on whether it has the banner to itself.
    //
    // One readout (the game clock alone) keeps the full width and the big clock,
    // and its label stays on one line -- a second line there would push the clock
    // out of the banner, which is how the clock used to vanish entirely.
    //
    // Two readouts split the width equally. The time then has a known space, so
    // its size is fixed rather than fitted; the label is the part that varies by
    // language, so it may wrap to two lines and shrinks to fit its half. Starting
    // the label at the smaller size is what guarantees two wrapped lines still fit
    // beside the time.
    //
    // `labels` lists every label across the banner so both settle on one size:
    // otherwise a short timeout label renders full-size beside a shrunken period
    // name and the two stop looking like a pair.
    struct Sizes {
        label: f32,
        time: f32,
    }
    let sizes = if snapshot.timeout.is_some() {
        Sizes {
            label: SMALL_TEXT,
            time: BANNER_TWO_TIME_TEXT,
        }
    } else {
        Sizes {
            label: SMALL_PLUS_TEXT,
            time: LARGE_TEXT,
        }
    };
    let wraps = snapshot.timeout.is_some();

    let make_time_view_row = |period_text,
                              time_text,
                              style: fn(&Theme) -> TextStyle,
                              labels: &[String],
                              times: &[String]| {
        let per = fit_text(period_text)
            .size(sizes.label)
            .min_size(BANNER_MIN_TEXT)
            .shared_with(labels.to_vec())
            .style(style)
            .align_x(Horizontal::Right)
            .height(Length::Shrink);
        let per = if wraps { per } else { per.no_wrap() };
        let time = fit_text(time_text)
            .no_wrap()
            .size(sizes.time)
            .min_size(BANNER_MIN_TEXT)
            .shared_with(times.to_vec())
            .style(style)
            .align_x(Horizontal::Left)
            .height(Length::Shrink);
        let r = row![].spacing(SPACING);
        make_time_view!(r, per, time).align_y(Alignment::Center)
    };

    let make_time_view_col = |period_text,
                              time_text,
                              style: fn(&Theme) -> TextStyle,
                              labels: &[String],
                              times: &[String]| {
        let per = fit_text(period_text)
            .size(sizes.label)
            .min_size(BANNER_MIN_TEXT)
            .shared_with(labels.to_vec())
            .style(style)
            .height(Length::Shrink);
        let per = if wraps { per } else { per.no_wrap() };
        let time = fit_text(time_text)
            .no_wrap()
            .size(sizes.time)
            .min_size(BANNER_MIN_TEXT)
            .shared_with(times.to_vec())
            .style(style)
            .height(Length::Shrink);
        let c = column![];
        make_time_view!(c, per, time).align_x(Alignment::Center)
    };

    // When the behind-schedule DELAY figure is showing, the period/clock and the
    // delay are stacked on two reduced-size lines, each laid out as
    // "label  value" (mirroring `make_time_view_row` so the two lines align into
    // a tidy column). Giving the clock its own line means a long period name
    // (e.g. "Second Half" or an overtime label) plus the delay figure can never
    // crowd the time out of the banner -- the root cause of the clock vanishing
    // in the second half with the delay figure enabled.
    let make_delay_line = |label: String, value: String, style: fn(&Theme) -> TextStyle| {
        let lab = container(
            text(label)
                .style(style)
                .size(SMALL_PLUS_TEXT)
                .width(Length::Shrink)
                .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Center);
        let val = text(value)
            .style(style)
            .size(MEDIUM_TEXT)
            .width(Length::Fill)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Left);
        row![lab, val]
            .spacing(SPACING)
            .width(Length::Fill)
            .align_y(Alignment::Center)
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

    let time_text = if schedule_ended {
        NO_TIME.to_string()
    } else {
        secs_to_long_time_string(snapshot.secs_in_period).to_string()
    };

    let time_text = time_text.trim().to_owned();

    // Behind schedule, the period/clock and DELAY share a two-line stacked
    // layout (handled here, before the normal one-line layouts). The delay is
    // hidden while a timeout is active -- the timeout takes that slot instead --
    // so this two-line layout only appears during normal play, with no third
    // column competing for width.
    if let Some(delay_value) = overrun_label.filter(|_| snapshot.timeout.is_none()) {
        let block = column![
            make_delay_line(period_text, time_text, period_color),
            make_delay_line(fl!("delay"), delay_value, red_text),
        ]
        .width(Length::Fill)
        .spacing(SPACING)
        .align_x(Alignment::Center);
        content = content.push(block);
    } else {
        // Collect every label and every time the banner is about to show, so both
        // readouts are fitted against the same strings and come out matching.
        let timeout_time = timeout_info.as_ref().map(|_| timeout_time_string(snapshot));
        let times: Vec<String> = [Some(time_text.clone()), timeout_time.clone()]
            .into_iter()
            .flatten()
            .collect();
        let labels: Vec<String> = [
            Some(period_text.clone()),
            timeout_info.as_ref().map(|(t, _)| t.clone()),
        ]
        .into_iter()
        .flatten()
        .collect();
        if tall {
            content = content.push(make_time_view_col(
                period_text,
                time_text,
                period_color,
                &labels,
                &times,
            ));
            if let Some(((timeout_text, timeout_color), timeout_time)) =
                timeout_info.zip(timeout_time)
            {
                content = content.push(make_time_view_col(
                    timeout_text,
                    timeout_time,
                    timeout_color,
                    &labels,
                    &times,
                ));
            }
        } else {
            content = content.push(make_time_view_row(
                period_text,
                time_text,
                period_color,
                &labels,
                &times,
            ));
            if let Some(((timeout_text, timeout_color), timeout_time)) =
                timeout_info.zip(timeout_time)
            {
                content = content.push(make_time_view_row(
                    timeout_text,
                    timeout_time,
                    timeout_color,
                    &labels,
                    &times,
                ));
            }
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
    uses_remote: bool,
    games: Option<&GameList>,
) -> (String, GameNumber) {
    let mut result = String::new();
    let game_number = if snapshot.current_period == GamePeriod::BetweenGames {
        let prev_game;
        let next_game;
        if uses_remote {
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
        if uses_remote {
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

/// Label for a config-page footer's red button: `CANCEL` when the page has
/// pending edits to discard, `BACK` when it does not. "Cancel" implies
/// discarding changes; with nothing to discard the button is plain navigation,
/// so it reads "Back". Driven off the page's *has-changes* predicate, NOT its
/// Apply-enabled flag — a page can hold pending changes that Apply still
/// refuses (e.g. an incomplete portal selection), and those must still read
/// "Cancel". Mirrors the cancel/back swap already used in `make_updates_page`.
/// The two language pickers do not use this — they label their buttons in the
/// *previewed* language via `Language::back_text()`.
pub(super) fn cancel_or_back_label(has_changes: bool) -> String {
    if has_changes {
        fl!("cancel")
    } else {
        fl!("back")
    }
}

/// A button that keeps `MIN_BUTTON_SIZE` whatever room the page has.
///
/// Two kinds of caller, and the second is easy to misread as a tile:
///
/// * Page furniture — footers, Back buttons — which must read at the same size
///   on every page.
/// * Body content that sits beside a *fixed* sibling from a helper this split
///   did not touch: the DARK/LIGHT team selectors, the team-timeout count
///   buttons, the TWO HALVES / ONE PERIOD control. They are only chrome because
///   `make_multi_label_button` and friends are still fixed — convert one to a
///   tile on its own and it grows away from the buttons beside it.
///
/// For a content cell whose neighbours already fill, use [`make_tile_button`].
pub(super) fn make_chrome_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    base_button(label).height(Length::Fixed(MIN_BUTTON_SIZE))
}

/// A large content cell in the body of a page, which takes an equal share of
/// whatever vertical room the page has.
///
/// Height is deliberately not a parameter: a tile beside another tile must be
/// the same height as it, and leaving that to each call site is what let the
/// MANUAL GAMES tile render short beside the source buttons on the Game Options
/// page.
///
/// Do NOT use this in a footer that declares no height of its own. `Row::push`
/// and `Column::push` run `Length::enclose`, which hands a `Shrink` container
/// its first filling child's length — so a tile there does not collapse; the
/// footer claims a share of the page and grows at the body's expense. Note the
/// two limits: `enclose` returns the *child's* length (a `FillPortion(2)` child
/// makes the container `FillPortion(2)`), and it runs at push time, so a later
/// `.height(...)` on the container wins. A container that already sets its own
/// height cannot grow — there, the reason to stay chrome is a fixed sibling to
/// match, not page height.
pub(super) fn make_tile_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    base_button(label).height(Length::Fill)
}

/// Shared construction for the two roles. Neither is implemented in terms of the
/// other: a property that belongs to one role must not reach the other by
/// accident.
fn base_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(label: T) -> Button<'a, Message> {
    button(fit_text(label)).padding(PADDING).width(Length::Fill)
}

pub(super) fn make_smaller_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    label: T,
) -> Button<'a, Message> {
    button(fit_text(label))
        .padding(PADDING)
        .height(Length::Fixed(XS_BUTTON_SIZE))
        .width(Length::Fill)
}

pub(super) fn make_multi_label_button<'a, Message: 'a + Clone, T: IntoFragment<'a>>(
    labels: (T, T),
) -> Button<'a, Message> {
    button(fit_text_lines(labels.0, labels.1))
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
}

/// The three faces the language picker draws in. Each tile draws in its own
/// language's script while the Cancel/Apply footer below draws in the
/// *selected* language's, so the two must resolve from the same three faces —
/// hence one definition here rather than one set per picker.
///
/// Written out in full rather than via `Font::with_name` because `LATIN_FONT`
/// needs `Weight::Medium`, which that constructor does not give it.
pub(super) const CJK_FONT: iced_core::Font = iced_core::Font {
    family: iced_core::font::Family::Name("WenQuanYi Zen Hei"),
    weight: iced_core::font::Weight::Normal,
    stretch: iced_core::font::Stretch::Normal,
    style: iced_core::font::Style::Normal,
};

pub(super) const THAI_FONT: iced_core::Font = iced_core::Font {
    family: iced_core::font::Family::Name("Noto Sans Thai"),
    weight: iced_core::font::Weight::Normal,
    stretch: iced_core::font::Stretch::Normal,
    style: iced_core::font::Style::Normal,
};

pub(super) const LATIN_FONT: iced_core::Font = iced_core::Font {
    family: iced_core::font::Family::Name("Roboto"),
    weight: iced_core::font::Weight::Medium,
    stretch: iced_core::font::Stretch::Normal,
    style: iced_core::font::Style::Normal,
};

/// The face a language is drawn in.
///
/// The three constants above are the only things that know what a typeface is
/// called; `Language::ui_font` is the only thing that knows which language uses
/// which. This is the seam between them, and the only way a tile gets a font --
/// so a tile cannot be given one that contradicts its own language, and cannot
/// be given none at all.
pub(super) fn font_for(lang: Language) -> iced_core::Font {
    match lang.ui_font() {
        UiFont::Cjk => CJK_FONT,
        UiFont::Thai => THAI_FONT,
        UiFont::Latin => LATIN_FONT,
    }
}

enum NameLines<T> {
    /// Name at the app-default text size. Used for short names like "TÜRKÇE".
    OneLine(T),
    /// Name at SMALL_TEXT. Used for long names like "BAHASA INDONESIA" that don't
    /// comfortably fit at the default size alongside the UNVERIFIED note below them.
    OneLineSmall(T),
}

/// A plain language tile: the native name, centred, in that language's script.
///
/// The sibling of [`make_lang_button_with_note`]. The two shapes sit next to
/// each other in the same row of the picker, so they are defined next to each
/// other here and neither takes a height — that is what keeps them in step.
/// Callers add the style and the message.
fn make_lang_button<'a, Message: 'a + Clone>(
    label: &'static str,
    font: iced_core::Font,
) -> Button<'a, Message> {
    let label_widget = centered_text(label).font(font);
    button(label_widget)
        .padding(PADDING)
        .height(Length::Fill)
        .width(Length::Fill)
}

/// A language tile with a small note beneath the name. Fills its row, and must
/// stay in step with [`make_lang_button`], the plain shape that sits beside it
/// in the same row: changing the height of one without the other is the MANUAL
/// GAMES defect again. Both are private and both are built by
/// [`make_language_grid_rows`], which is what makes that possible to check.
fn make_lang_button_with_note<'a, Message, T>(
    main: NameLines<T>,
    note: T,
    font: iced_core::Font,
) -> Button<'a, Message>
where
    Message: 'a + Clone,
    T: IntoFragment<'a>,
{
    let with_font = |t: Text<'a>| -> Text<'a> { t.font(font) };
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
        .height(Length::Fill)
        .width(Length::Fill)
}

/// The 15 language tiles as four rows, shared by both language pickers.
///
/// Both pickers send `Message::SelectLanguage`; what differs between them is
/// only what surrounds the grid — the ribbon, filler rows and footer that each
/// page adds for itself. So this returns the rows, not the page, and each page's
/// own comment owns its layout.
///
/// Returned as exactly four rows because the grid is hand-written, not
/// generated: a sixteenth language needs a considered place in the alphabetical
/// order, not an automatic append.
pub(super) fn make_language_grid_rows<'a>(selected: Language) -> [Element<'a, Message>; 4] {
    let lang_btn = |lang: Language, label: &'static str| -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_lang_button(label, font_for(lang))
            .style(style)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Button variant for unverified translations: shows native name plus a small
    // "(UNVERIFIED)" note in that language's own script. The note text is hardcoded
    // in each target language, not routed through fl!, because fl! always renders
    // in the operator's current locale — but each button must label itself.
    let lang_btn_note = |lang: Language,
                         main: NameLines<&'static str>,
                         note: &'static str|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_lang_button_with_note(main, note, font_for(lang))
            .style(style)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Languages sorted alphabetically by romanized native name:
    // Bahasa Indonesia(B), Bahasa Melayu(B), Deutsch(D), English(E),
    // Español(E), Filipino(F), Français(F), Hangugeo/한국어(H), Italiano(I),
    // Nederlands(N), Nihongo/日本語(N), Português(P), Thai/ภาษาไทย(T),
    // Türkçe(T), Zhōngwén/中文(Z)
    //
    // English, Spanish, and French are considered verified. Every other language
    // gets a small "(UNVERIFIED)" note in its own language, signalling to operators
    // that a native speaker has not yet reviewed the translation.
    [
        row![
            lang_btn_note(
                Language::Indonesian,
                NameLines::OneLineSmall("BAHASA INDONESIA"),
                "(BELUM DIVERIFIKASI)",
            ),
            lang_btn_note(
                Language::Malay,
                NameLines::OneLineSmall("BAHASA MELAYU"),
                "(BELUM DISAHKAN)",
            ),
            lang_btn_note(
                Language::German,
                NameLines::OneLine("DEUTSCH"),
                "(NICHT VERIFIZIERT)",
            ),
            lang_btn(Language::English, "ENGLISH"),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into(),
        row![
            lang_btn(Language::Spanish, "ESPAÑOL"),
            lang_btn_note(
                Language::Tagalog,
                NameLines::OneLine("FILIPINO"),
                "(HINDI PA NA-VERIFY)",
            ),
            lang_btn(Language::French, "FRANÇAIS"),
            lang_btn_note(
                Language::Korean,
                NameLines::OneLine("한국어"),
                "(검증되지 않음)",
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into(),
        row![
            lang_btn_note(
                Language::Italian,
                NameLines::OneLine("ITALIANO"),
                "(NON VERIFICATO)",
            ),
            lang_btn_note(
                Language::Dutch,
                NameLines::OneLine("NEDERLANDS"),
                "(NIET GEVERIFIEERD)",
            ),
            lang_btn_note(Language::Japanese, NameLines::OneLine("日本語"), "(未検証)"),
            lang_btn_note(
                Language::Portuguese,
                NameLines::OneLine("PORTUGUÊS"),
                "(NÃO VERIFICADO)",
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into(),
        row![
            lang_btn_note(
                Language::Thai,
                NameLines::OneLine("ภาษาไทย"),
                "(ยังไม่ได้ตรวจสอบ)",
            ),
            lang_btn_note(
                Language::Turkish,
                NameLines::OneLine("TÜRKÇE"),
                "(DOĞRULANMAMIŞ)",
            ),
            lang_btn_note(Language::Mandarin, NameLines::OneLine("中文"), "(未验证)"),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into(),
    ]
}

/// The 12 buzzer sounds as three rows of four, shared by both buzzer pickers.
///
/// `on_select` is the message each cell sends: the Sound settings page passes
/// `Message::SelectBuzzer`, the beep-test picker passes
/// `Message::BeepTestSelectBuzzer`. The two pickers are deliberately separate
/// pages sending separate messages — only the cells are shared.
///
/// Height is not a parameter, for the reason given on [`make_tile_button`].
/// Callers place these rows in their own column and add their own filler rows
/// and footer, which is all that differs between the two pages.
///
/// **Adding a 13th `BuzzerSound` changes both pages' layout.** `chunks(4)` over
/// `BuzzerSound::ALL` yields three rows today, and each caller picks its number
/// of filler rows to balance against exactly three; a fourth row would unbalance
/// both without any build failure. This returns a `Vec` rather than the
/// `[Element; 3]` that would make it a compile error, because the row count is
/// derived from a slice and a fixed-size conversion would need a fallible cast
/// this function has no way to fail. Revisit both pickers' filler rows if the
/// sound list ever changes length.
pub(super) fn make_buzzer_grid_rows<'a>(
    selected: BuzzerSound,
    on_select: fn(BuzzerSound) -> Message,
) -> Vec<Element<'a, Message>> {
    // Build each sound cell: blue when selected, gray otherwise.
    let cell = |s: BuzzerSound| -> Element<'a, Message> {
        let style = if s == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        button(centered_text(s.to_string().to_uppercase()))
            .padding(PADDING)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(style)
            .on_press(on_select(s))
            .into()
    };

    // 12 sounds laid out in 3 rows of 4. BuzzerSound::ALL is always exactly 12
    // elements.
    let mut rows = Vec::new();
    for chunk in BuzzerSound::ALL.chunks(4) {
        let mut r = Row::new().spacing(SPACING).height(Length::Fill);
        for &s in chunk {
            r = r.push(cell(s));
        }
        // Pad any short final chunk with spacers (chunks(4) on 12 items is always
        // exactly 3 full rows, but this keeps the layout robust).
        for _ in chunk.len()..4 {
            r = r.push(horizontal_space());
        }
        rows.push(r.into());
    }
    rows
}

/// Text centred in both axes, filling its parent.
///
/// **Do not use this for a label that changes in place.** Pairing
/// `align_y(Center)` with `height(Fill)` is the iced 0.13 stale-paragraph
/// pattern that `make_multi_label_button` and the other button helpers were
/// rewritten to avoid — and this helper does the same on the horizontal axis, so
/// both of its anchors depend on the content. A text widget's cached paragraph
/// position survives a content change, so the replacement label is drawn from
/// the old one's offset and leaves the previous glyphs stranded on screen.
/// `make_custom_site_page`'s rejection message carries a longer note on the same
/// class of bug, measured on screen, and avoids this helper for exactly that
/// reason. The title on that same page still uses it — see below.
///
/// It is safe at every call site today only because none of those labels ever
/// changes while its widget stays put: the buzzer cells, the language tiles and
/// the beep-test preset button are fixed per cell — selecting one changes the
/// *style*, never the text — and the foul keypad's `=` is a literal. The two
/// built from `fl!` (the preset button's label and the custom-site title) change
/// only on a language switch, which rebuilds the page. Making any of these
/// labels dynamic means moving it off this helper first.
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
    button(fit_text(label).size(size))
        .width(Length::Fixed(MIN_BUTTON_SIZE))
        .height(Length::Fixed(MIN_BUTTON_SIZE))
}

/// A label-and-value tile in the body of a page. Fills its row, like every
/// other tile — see [`make_tile_button`].
///
/// Where a label-and-value button is furniture rather than a tile, use
/// [`make_value_chrome_button`] instead — see its doc for the current list.
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
            // Label and value each get a guaranteed share of the width and
            // wrap or shrink inside it. Letting either take what it wants first
            // starves the other: the value used to be clipped to "1/" because
            // the label claimed the row, and giving the value priority instead
            // let "1/HALBZEIT" at the large size crowd out the label.
            //
            // Do NOT pair `align_y(Center)` with `height(Fill)` here: that caches
            // a paragraph-position anchor that bleeds across renders (iced 0.13
            // bug; see portal_detail::row_text_centered and the time-view fix in
            // this file). The row handles the vertical centering instead — see
            // the comment on its `height(Fill)` below.
            fit_text(first_label)
                .size(if large_text.0 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .align_x(Horizontal::Left)
                .width(Length::FillPortion(3))
                .height(Length::Shrink),
            fit_text(second_label)
                .size(if large_text.1 {
                    MEDIUM_TEXT
                } else {
                    SMALL_TEXT
                })
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(2))
                .height(Length::Shrink),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center)
        // The row must fill the button. `iced_core::layout::flex` seeds the cross
        // axis at 0 for a Shrink-height row, grows it only to the tallest child,
        // and then centres the children in *that* band — which sits at the top
        // padding. A Shrink row therefore leaves the label riding high in a
        // fixed-height button, and stranded at the top of a filling one.
        //
        // This is not confined to filling tiles: it also moves the label down a
        // few pixels in every fixed-height value button, bringing them into line
        // with `make_chrome_button`, whose `fit_text` already centres itself.
        .height(Length::Fill)
        .padding(PADDING),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .style(light_gray_button);

    if let Some(message) = message {
        button = button.on_press(message);
    }
    button
}

/// The fixed-height counterpart to [`make_value_button`], for a label-and-value
/// button that is page furniture rather than a tile. Three call sites: the Game
/// Options footer's game picker, the brightness button beside the display
/// preview, and the Updates page's version row.
///
/// The Game Options footer declares no height, so a filling child there would
/// make the whole footer claim a share of the page and grow — see
/// [`make_tile_button`]. The other two containers set their own height, so they
/// cannot grow; those two stay fixed for the simpler reason that each has a
/// fixed sibling to match (OPEN NEW DISPLAY, CHECK FOR UPDATES). Filling one
/// without the other is the MANUAL GAMES defect again.
pub(super) fn make_value_chrome_button<'a, T, U>(
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
    make_value_button(first_label, second_label, large_text, message)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
}

/// A value row whose value is long and whose label is short — the custom site's
/// address is the only one so far.
///
/// Two differences from `make_value_button`, both of which a web address needs:
///
/// * The shares are reversed, so the value gets the larger one. The 3:2 split
///   there suits a long label beside a short value (`OT HALF TIME LENGTH: 5:00`);
///   a four-letter label beside a URL is the same row backwards, and it left
///   half the row empty while the address wrapped in the remaining third.
/// * The value shrinks rather than wrapping, because `best_split` breaks after a
///   `/`. That is right for `1/HALBZEIT` and wrong for an address, where a break
///   mid-path reads as a character that is not there.
///
/// Shares rather than `Shrink` for the value: `FitText::layout` fits to
/// `limits.max()`, so a `Shrink` value is measured against the whole row and a
/// long enough address would crowd the label out entirely — the failure the
/// comment in `make_value_button` records.
pub(super) fn make_long_value_button<'a, T, U>(
    first_label: T,
    second_label: U,
    message: Option<Message>,
) -> Button<'a, Message>
where
    Message: 'a + Clone,
    T: IntoFragment<'a>,
    U: IntoFragment<'a>,
{
    let mut button = button(
        row![
            fit_text(first_label)
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Left)
                .width(Length::FillPortion(2))
                .height(Length::Shrink),
            fit_text(second_label)
                .size(MEDIUM_TEXT)
                .no_wrap()
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(3))
                .height(Length::Shrink),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center)
        // Fills the button for the same reason as `make_value_button` — see the
        // comment there.
        .height(Length::Fill)
        .padding(PADDING),
    )
    .height(Length::Fill)
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
        row![text(infraction_bar_label(infraction))]
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

/// Text for the infraction picker's header bar. Until an infraction is chosen
/// the bar prompts for one rather than naming `Unknown`, which would read as a
/// real choice — the Add Foul and Add Warning pages are the only callers that
/// show this bar, and both refuse to save without a selection (see
/// `foul_add_can_commit` / `warning_add_can_commit`). `unknown` stays the
/// wording for saved list rows, where a penalty legitimately can carry no
/// infraction (fouls-and-warnings tracking off). The prompt is substituted into
/// the same `infraction` template as a real name so the localized prefix always
/// matches the populated state.
fn infraction_bar_label(infraction: Infraction) -> String {
    let value = if infraction == Infraction::Unknown {
        fl!("select-infraction")
    } else {
        inf_short_name(infraction)
    };
    fl!("infraction", infraction = value)
}

/// Returns true when any of the given format hints represents a pending change
/// (anything other than `NoChange`). Used to gray the Apply button on the
/// penalty / warning / foul overview pages until a row is added, edited, or
/// deleted.
pub(super) fn any_pending_change(hints: impl IntoIterator<Item = FormatHint>) -> bool {
    hints.into_iter().any(|h| h != FormatHint::NoChange)
}

#[cfg(test)]
mod tests {
    /// The seam between "which typeface" and "what that typeface is called".
    /// One language per face, and each named family asserted, so a face wired
    /// to the wrong group -- which no compiler would notice -- fails here.
    #[test]
    fn each_language_resolves_to_its_own_bundled_face() {
        use iced_core::font::Family;
        for (lang, expected) in [
            (Language::Japanese, "WenQuanYi Zen Hei"),
            (Language::Korean, "WenQuanYi Zen Hei"),
            (Language::Mandarin, "WenQuanYi Zen Hei"),
            (Language::Thai, "Noto Sans Thai"),
            (Language::English, "Roboto"),
            (Language::Spanish, "Roboto"),
            (Language::French, "Roboto"),
            (Language::Turkish, "Roboto"),
        ] {
            assert_eq!(
                font_for(lang).family,
                Family::Name(expected),
                "{lang:?} resolved to the wrong bundled face"
            );
        }
    }

    use super::*;
    use crate::config::Mode;

    #[test]
    fn any_pending_change_detects_edits() {
        let empty: [FormatHint; 0] = [];
        assert!(!any_pending_change(empty));
        assert!(!any_pending_change([
            FormatHint::NoChange,
            FormatHint::NoChange
        ]));
        assert!(any_pending_change([
            FormatHint::NoChange,
            FormatHint::Edited
        ]));
        assert!(any_pending_change([FormatHint::New]));
        assert!(any_pending_change([FormatHint::Deleted]));
    }

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

    #[test]
    fn cancel_or_back_label_swaps_on_changes() {
        // Pending changes → the "cancel" label; nothing to discard → the "back" label.
        // Compared against the same fl! keys so the assertion holds regardless of which
        // locale the loader resolves to.
        assert_eq!(cancel_or_back_label(true), fl!("cancel"));
        assert_eq!(cancel_or_back_label(false), fl!("back"));
        assert_ne!(cancel_or_back_label(true), cancel_or_back_label(false));
    }

    #[test]
    fn infraction_bar_prompts_until_a_selection_is_made() {
        // Nothing picked yet → the bar prompts for a choice instead of naming
        // "Unknown" as though it were one. Compared against the same fl! keys so
        // the assertion holds regardless of which locale the loader resolves to.
        assert_eq!(
            infraction_bar_label(Infraction::Unknown),
            fl!("infraction", infraction = fl!("select-infraction"))
        );
        // Would have passed before this fix, and must not again.
        assert_ne!(
            infraction_bar_label(Infraction::Unknown),
            fl!("infraction", infraction = fl!("unknown"))
        );
    }

    #[test]
    fn infraction_bar_names_a_chosen_infraction() {
        for inf in all::<Infraction>().filter(|i| *i != Infraction::Unknown) {
            assert_eq!(
                infraction_bar_label(inf),
                fl!("infraction", infraction = inf_short_name(inf))
            );
        }
    }

    #[test]
    fn make_scroll_list_handles_index_past_end_of_shrunken_list() {
        // Regression for the portal-detail crash (H7): a list can shrink under
        // a remembered scroll position (queued games uploading in the
        // background) so the index points past the new end. The scrollbar math
        // `num_items - LIST_LEN - index` must not underflow (a debug panic /
        // release u16 wrap). num_items=5 (> LIST_LEN=4, so it enters the
        // subtraction branch) with index=6 (> num_items - LIST_LEN = 1) is the
        // exact trigger; before the clamp this panicked here in debug builds.
        const LIST_LEN: usize = 4;
        let buttons: [Element<'static, Message>; LIST_LEN] =
            core::array::from_fn(|_| horizontal_space().into());
        let _list = make_scroll_list::<LIST_LEN>(
            buttons,
            5,
            6,
            text("test"),
            ScrollOption::PortalDetail,
            transparent_container,
        );
    }
}
