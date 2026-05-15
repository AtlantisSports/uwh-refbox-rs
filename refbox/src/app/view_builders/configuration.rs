use super::{ViewData, fl, message::*, shared_elements::*, theme::*};
use crate::app::PageEntrySnapshot;
use crate::app::languages::Language;
use crate::config::Mode;
use crate::sound_controller::*;
use collect_array::CollectArrayResult;
use iced::{
    Alignment, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{button, column, container, horizontal_space, row, text, vertical_space},
};
use matrix_drawing::transmitted_data::Brightness;
use std::collections::BTreeMap;
use tokio::time::Duration;
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::GameSnapshot,
    uwhportal::schedule::{Event, EventId, GameNumber, Schedule},
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in super::super) struct EditableSettings {
    pub config: GameConfig,
    pub game_number: GameNumber,
    pub white_on_right: bool,
    pub brightness: Brightness,
    pub using_uwhportal: bool,
    pub uwhportal_token_valid: Option<bool>,
    pub current_event_id: Option<EventId>,
    pub current_court: Option<String>,
    pub schedule: Option<Schedule>,
    pub sound: SoundSettings,
    pub mode: Mode,
    pub hide_time: bool,
    pub collect_scorer_cap_num: bool,
    pub track_fouls_and_warnings: bool,
    pub confirm_score: bool,
    pub pending_language: Option<Language>,
    pub original_language: Option<Language>,
}

impl EditableSettings {
    /// Returns `true` when portal mode is engaged but the configuration is not
    /// yet committable: event/court/schedule still missing, the chosen game number
    /// isn't in the schedule, or the chosen game's court doesn't match the
    /// currently-selected court.
    ///
    /// Both `apply_game_options` (gating the actual commit) and
    /// `make_cancel_apply_footer` (disabling Apply when nothing is committable)
    /// rely on this predicate, so they stay in sync.
    pub(in super::super) fn uwhportal_incomplete(&self) -> bool {
        if !self.using_uwhportal {
            return false;
        }
        if self.current_event_id.is_none()
            || self.current_court.is_none()
            || self.schedule.is_none()
        {
            return true;
        }
        // Safety: guarded by the is_none() check on lines 55-58 above; reachable only when both schedule and current_court are Some.
        match self.schedule.as_ref().unwrap().games.get(&self.game_number) {
            Some(g) => g.court != *self.current_court.as_ref().unwrap(),
            None => true,
        }
    }

    /// Record an event-picker selection. Sets the new event id and clears any
    /// court / game-number / schedule that was filtered by the previous event so
    /// the user re-picks against the new event's data.
    pub(in super::super) fn select_event(&mut self, id: EventId) {
        self.current_event_id = Some(id);
        self.current_court = None;
        self.game_number = String::new();
        self.schedule = None;
    }

    /// Record a court-picker selection. Sets the new court and clears the
    /// game number so the user re-picks from the new court's filtered list.
    pub(in super::super) fn select_court(&mut self, court: String) {
        self.current_court = Some(court);
        self.game_number = String::new();
    }
}

pub(in super::super) trait Cyclable
where
    Self: Sized,
{
    fn next(&self) -> Self;

    fn cycle(&mut self) {
        *self = self.next();
    }
}

impl Cyclable for BuzzerSound {
    fn next(&self) -> Self {
        match self {
            Self::Buzz => Self::Whoop,
            Self::Whoop => Self::Crazy,
            Self::Crazy => Self::DeDeDu,
            Self::DeDeDu => Self::TwoTone,
            Self::TwoTone => Self::Buzz,
        }
    }
}

impl Cyclable for Option<BuzzerSound> {
    fn next(&self) -> Self {
        match self {
            Some(BuzzerSound::Buzz) => Some(BuzzerSound::Whoop),
            Some(BuzzerSound::Whoop) => Some(BuzzerSound::Crazy),
            Some(BuzzerSound::Crazy) => Some(BuzzerSound::DeDeDu),
            Some(BuzzerSound::DeDeDu) => Some(BuzzerSound::TwoTone),
            Some(BuzzerSound::TwoTone) => None,
            None => Some(BuzzerSound::Buzz),
        }
    }
}

impl Cyclable for Volume {
    fn next(&self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Max,
            Self::Max => Self::Off,
        }
    }
}

impl Cyclable for Mode {
    fn next(&self) -> Self {
        match self {
            Self::Hockey6V6 => Self::Hockey3V3,
            Self::Hockey3V3 => Self::Rugby,
            Self::Rugby => Self::Hockey6V6,
        }
    }
}

impl Cyclable for Brightness {
    fn next(&self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Outdoor,
            Self::Outdoor => Self::Low,
        }
    }
}

pub(in super::super) fn page_has_changes(
    page: ConfigPage,
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
) -> bool {
    let Some(snapshot) = snapshot else {
        return false;
    };
    match (page, snapshot) {
        (
            ConfigPage::Game,
            PageEntrySnapshot::Game {
                config,
                game_number,
                using_uwhportal,
                current_event_id,
                current_court,
                schedule,
            },
        ) => {
            edited.config != *config
                || edited.game_number != *game_number
                || edited.using_uwhportal != *using_uwhportal
                || edited.current_event_id != *current_event_id
                || edited.current_court != *current_court
                || edited.schedule != *schedule
        }
        (
            ConfigPage::App,
            PageEntrySnapshot::App {
                using_uwhportal,
                current_event_id,
                current_court,
                schedule,
                mode,
                collect_scorer_cap_num,
                track_fouls_and_warnings,
                confirm_score,
            },
        ) => {
            edited.using_uwhportal != *using_uwhportal
                || edited.current_event_id != *current_event_id
                || edited.current_court != *current_court
                || edited.schedule != *schedule
                || edited.mode != *mode
                || edited.collect_scorer_cap_num != *collect_scorer_cap_num
                || edited.track_fouls_and_warnings != *track_fouls_and_warnings
                || edited.confirm_score != *confirm_score
        }
        (
            ConfigPage::Display,
            PageEntrySnapshot::Display {
                white_on_right,
                brightness,
                hide_time,
            },
        ) => {
            edited.white_on_right != *white_on_right
                || edited.brightness != *brightness
                || edited.hide_time != *hide_time
        }
        (ConfigPage::Sound, PageEntrySnapshot::Sound { sound }) => edited.sound != *sound,
        (ConfigPage::Remotes(_, _), PageEntrySnapshot::Remotes { remotes }) => {
            edited.sound.remotes != *remotes
        }
        (
            ConfigPage::Language,
            PageEntrySnapshot::Language {
                original_language,
                pending_language,
            },
        ) => {
            edited.original_language != *original_language
                || edited.pending_language != *pending_language
        }
        _ => false,
    }
}

pub(in super::super) fn build_game_config_edit_page<'a>(
    data: ViewData<'_, '_>,
    settings: &EditableSettings,
    events: Option<&BTreeMap<EventId, Event>>,
    page: ConfigPage,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    match page {
        ConfigPage::Main => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Game => make_event_config_page(
            snapshot,
            settings,
            events,
            mode,
            clock_running,
            page_entry_snapshot,
        ),
        ConfigPage::Sound => {
            make_sound_config_page(snapshot, settings, mode, clock_running, page_entry_snapshot)
        }
        ConfigPage::Display => {
            make_display_config_page(snapshot, settings, mode, clock_running, page_entry_snapshot)
        }
        ConfigPage::App => {
            make_app_config_page(mode, snapshot, settings, clock_running, page_entry_snapshot)
        }
        ConfigPage::User => make_user_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Remotes(index, listening) => make_remote_config_page(
            snapshot,
            settings,
            index,
            listening,
            mode,
            clock_running,
            page_entry_snapshot,
        ),
        ConfigPage::Language => {
            make_language_select_page(snapshot, settings, mode, clock_running, page_entry_snapshot)
        }
    }
}

fn make_main_config_page<'a>(
    snapshot: &GameSnapshot,
    _settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let row_top = row![
        make_button(fl!("game-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Game)),
        make_button(fl!("app-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::App)),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    let row_bottom = row![
        make_button(fl!("user-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::User)),
        make_button(fl!("language"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Language)),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row_top,
        row_bottom,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![
            make_back_button(Message::ConfigEditComplete),
            horizontal_space(),
            horizontal_space(),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_back_button<'a>(destination: Message) -> Element<'a, Message> {
    make_button(fl!("back"))
        .style(red_button)
        .on_press(destination)
        .into()
}

fn make_cancel_apply_footer<'a>(
    page: ConfigPage,
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    // Apply is enabled when there are pending changes AND the resulting state is
    // committable. For Game Options in portal mode, "committable" requires a
    // complete portal selection (event + court + schedule + game-in-schedule);
    // otherwise pressing Apply would only open a wasteful "fix something and try
    // again" dialog. Other pages have no committability gate.
    let apply_blocked = matches!(page, ConfigPage::Game) && edited.uwhportal_incomplete();
    let apply_enabled = page_has_changes(page, edited, snapshot) && !apply_blocked;

    let cancel = make_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(page));

    let apply = make_button(fl!("apply"))
        .style(green_button)
        .width(Length::Fill);
    let apply = if apply_enabled {
        apply.on_press(Message::ApplyConfigPage(page))
    } else {
        apply
    };

    row![cancel, horizontal_space(), apply]
        .spacing(SPACING)
        .into()
}

fn make_user_config_page<'a>(
    snapshot: &GameSnapshot,
    _settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    // Hidden spacer reserved for a future view-mode tile so the row keeps three equal columns.
    let view_mode_spacer = horizontal_space().width(Length::Fill);

    let tiles = row![
        make_button(fl!("display-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Display)),
        view_mode_spacer,
        make_button(fl!("sound-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Sound)),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        tiles,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![
            make_back_button(Message::ChangeConfigPage(ConfigPage::Main)),
            horizontal_space(),
            horizontal_space(),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

// View builder takes app-state slices; grouping into a context struct is a separate refactor across all view_builders. Filed as a Findings-Backlog item in AUDIT-PLAN.md (Unit 3, 2026-05-13).
#[allow(clippy::too_many_arguments)]
fn make_event_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    events: Option<&BTreeMap<EventId, Event>>,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let EditableSettings {
        config,
        game_number,
        using_uwhportal,
        current_event_id,
        current_court,
        schedule,
        ..
    } = settings;

    let using_uwhportal = *using_uwhportal;

    // Game-number picker — placed in the centre cell of the action row
    // (Cancel | Game | Apply) in both portal modes per ADR-009 Task 14 layout.
    let game_btn_msg = if using_uwhportal {
        if current_event_id.is_some() && current_court.is_some() && schedule.is_some() {
            Some(Message::SelectParameter(ListableParameter::Game))
        } else {
            None
        }
    } else {
        Some(Message::KeypadPage(KeypadPage::GameNumber))
    };

    let mut game_large_text = true;
    let game_label = if using_uwhportal {
        if let (Some(_), Some(cur_court)) = (current_event_id, current_court) {
            if let Some(schedule) = schedule {
                match schedule.games.get(game_number) {
                    Some(game) if game.court == *cur_court => game.number.to_string(),
                    _ => {
                        game_large_text = false;
                        fl!("none-selected")
                    }
                }
            } else {
                fl!("loading")
            }
        } else {
            String::new()
        }
    } else {
        game_number.to_string()
    };

    // Using UWH Portal toggle — row 1 left cell in both portal modes.
    let using_uwh_portal_btn = make_value_button(
        fl!("using-uwh-portal"),
        bool_string(using_uwhportal),
        (false, true),
        Some(Message::ToggleBoolParameter(
            BoolGameParameter::UsingUwhPortal,
        )),
    );

    // Column layout: page_content fills available height between the top
    // game-time button and the bottom timeout ribbon. Data rows take Fill
    // height so they each absorb an equal share of the leftover vertical
    // space, giving uniform inter-row gaps with the action row sitting just
    // above the timeout ribbon. Action row stays at MIN_BUTTON_SIZE so the
    // Cancel/Game/Apply chrome reads at a consistent size across pages.
    let mut col = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
    )]
    .spacing(SPACING)
    .height(Length::Fill);

    if using_uwhportal {
        // Portal mode ON: row 1 = UWH Portal + 2 blanks; rows 2–4 = full-width
        // Event / Token / Court single-button rows.
        let event_label = if let Some(events) = events {
            if let Some(event_id) = current_event_id {
                match events.get(event_id) {
                    Some(t) => t.name.clone(),
                    None => fl!("none-selected"),
                }
            } else {
                fl!("none-selected")
            }
        } else {
            fl!("loading")
        };

        let event_btn_msg = if events.is_some() {
            Some(Message::SelectParameter(ListableParameter::Event))
        } else {
            None
        };

        let pool_label = if let Some(event) = events
            .as_ref()
            .and_then(|events| events.get(current_event_id.as_ref()?))
        {
            if event.courts.is_some() {
                if let Some(court) = current_court {
                    court.clone()
                } else {
                    fl!("none-selected")
                }
            } else {
                fl!("loading")
            }
        } else {
            String::new()
        };

        let pool_btn_msg = events
            .as_ref()
            .and_then(|tourns| tourns.get(current_event_id.as_ref()?)?.courts.as_ref())
            .map(|_| Message::SelectParameter(ListableParameter::Court));

        let auth_container = |auth| {
            let txt = match auth {
                Some(true) => "OK",
                Some(false) => "FAILED",
                None => "CHECKING...",
            };
            let style = match auth {
                Some(true) => green_container,
                Some(false) => red_container,
                None => gray_container,
            };
            container(txt)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(style)
        };

        let uwhportal_auth_text = text("UWHPORTAL TOKEN:")
            .size(MEDIUM_TEXT)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Right)
            .width(Length::Fill)
            .height(Length::Fill);

        let auth_state_message = if settings.current_event_id.is_some() {
            Some(Message::KeypadPage(KeypadPage::PortalLogin(0, false)))
        } else {
            None
        };

        let auth_state_button = button(
            row![
                uwhportal_auth_text,
                auth_container(settings.uwhportal_token_valid),
            ]
            .padding(PADDING)
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(0)
        .style(light_gray_button)
        .on_press_maybe(auth_state_message);

        col = col
            .push(
                row![using_uwh_portal_btn, horizontal_space(), horizontal_space()]
                    .spacing(SPACING)
                    .height(Length::Fill),
            )
            .push(
                make_value_button(fl!("event"), event_label, (true, true), event_btn_msg)
                    .height(Length::Fill),
            )
            .push(auth_state_button)
            .push(
                make_value_button(fl!("court"), pool_label, (true, true), pool_btn_msg)
                    .height(Length::Fill),
            );
    } else {
        // Portal mode OFF: 4 data rows × 3 cells each.
        col = col
            .push(
                row![
                    using_uwh_portal_btn,
                    make_value_button(
                        fl!("overtime-allowed"),
                        bool_string(config.overtime_allowed),
                        (false, true),
                        Some(Message::ToggleBoolParameter(
                            BoolGameParameter::OvertimeAllowed,
                        )),
                    ),
                    make_value_button(
                        fl!("sudden-death-allowed"),
                        bool_string(config.sudden_death_allowed),
                        (false, true),
                        Some(Message::ToggleBoolParameter(
                            BoolGameParameter::SuddenDeathAllowed,
                        )),
                    )
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            )
            .push(
                row![
                    make_value_button(
                        if config.single_half {
                            fl!("game-length")
                        } else {
                            fl!("half-length-full")
                        },
                        time_string(config.half_play_duration),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::Half)),
                    ),
                    make_value_button(
                        fl!("pre-ot-break-length"),
                        time_string(config.pre_overtime_break),
                        (false, true),
                        if config.overtime_allowed {
                            Some(Message::EditParameter(LengthParameter::PreOvertime))
                        } else {
                            None
                        },
                    ),
                    make_value_button(
                        fl!("pre-sd-break-length"),
                        time_string(config.pre_sudden_death_duration),
                        (false, true),
                        if config.sudden_death_allowed {
                            Some(Message::EditParameter(LengthParameter::PreSuddenDeath))
                        } else {
                            None
                        },
                    )
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            )
            .push(
                row![
                    make_value_button(
                        fl!("half-time-length"),
                        time_string(config.half_time_duration),
                        (false, true),
                        if !config.single_half {
                            Some(Message::EditParameter(LengthParameter::HalfTime))
                        } else {
                            None
                        },
                    ),
                    make_value_button(
                        fl!("ot-half-length"),
                        time_string(config.ot_half_play_duration),
                        (false, true),
                        if config.overtime_allowed {
                            Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                        } else {
                            None
                        },
                    ),
                    make_value_button(
                        fl!("minimum-brk-btwn-games"),
                        time_string(config.minimum_break),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                    )
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            )
            .push(
                row![
                    make_value_button(
                        if config.timeouts_counted_per_half {
                            fl!("num-tos-per-half")
                        } else {
                            fl!("num-tos-per-game")
                        },
                        config.num_team_timeouts_allowed.to_string(),
                        (false, true),
                        Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                            config.team_timeout_duration,
                            config.timeouts_counted_per_half,
                        ))),
                    ),
                    make_value_button(
                        fl!("ot-half-time-length"),
                        time_string(config.ot_half_time_duration),
                        (false, true),
                        if config.overtime_allowed {
                            Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
                        } else {
                            None
                        },
                    ),
                    make_value_button(
                        fl!("nominal-break-between-games"),
                        time_string(config.nominal_break),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                    )
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            );
    }

    // Action row: Cancel | Game-number picker | Apply.
    // Apply is blocked when the portal state is incomplete (carried over from
    // make_cancel_apply_footer's gate so a click on Apply can't reach a
    // wasteful confirmation dialog).
    let apply_blocked = settings.uwhportal_incomplete();
    let apply_enabled =
        page_has_changes(ConfigPage::Game, settings, page_entry_snapshot) && !apply_blocked;

    let cancel_btn = make_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::Game));

    let game_picker_btn = make_value_button(
        fl!("game-select"),
        game_label,
        (false, game_large_text),
        game_btn_msg,
    );

    let apply_btn = make_button(fl!("apply"))
        .style(green_button)
        .width(Length::Fill);
    let apply_btn = if apply_enabled {
        apply_btn.on_press(Message::ApplyConfigPage(ConfigPage::Game))
    } else {
        apply_btn
    };

    col = col.push(row![cancel_btn, game_picker_btn, apply_btn].spacing(SPACING));

    col.into()
}

fn make_app_config_page<'a>(
    mode: Mode,
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let EditableSettings {
        collect_scorer_cap_num,
        track_fouls_and_warnings,
        confirm_score,
        ..
    } = settings;

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        row![
            make_value_button(
                fl!("app-mode"),
                settings.mode.to_string(),
                (false, true),
                Some(Message::CycleParameter(CyclingParameter::Mode)),
            ),
            make_value_button(
                fl!("track-cap-number-of-scorer"),
                bool_string(*collect_scorer_cap_num),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ScorerCapNum,
                )),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("track-fouls-and-warnings"),
                bool_string(*track_fouls_and_warnings),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::FoulsAndWarnings,
                )),
            ),
            make_value_button(
                fl!("confirm-score-at-game-end"),
                bool_string(*confirm_score),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ConfirmScore,
                )),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        make_cancel_apply_footer(ConfigPage::App, settings, page_entry_snapshot),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_display_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let EditableSettings {
        white_on_right,
        hide_time,
        brightness,
        ..
    } = settings;

    let white = container(text(fl!("light-team-name-caps")))
        .center_x(Length::FillPortion(2))
        .center_y(Length::Fill)
        .style(white_container);
    let black = container(text(fl!("dark-team-name-caps")))
        .center_x(Length::FillPortion(2))
        .center_y(Length::Fill)
        .style(black_container);

    let center = text(fl!("starting-sides"))
        .size(MEDIUM_TEXT)
        .align_y(Vertical::Center)
        .align_x(Horizontal::Center)
        .width(Length::FillPortion(3));

    // `white_on_right` is based on the view from the front of the panels, so for the ref's point
    // of view we need to reverse the direction
    let sides = if *white_on_right {
        // White to Ref's left
        row![white, center, black].padding(PADDING)
    } else {
        // White to Ref's right
        row![black, center, white].padding(PADDING)
    };

    let sides_btn = button(sides.width(Length::Fill).height(Length::Fill))
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .padding(0)
        .style(light_gray_button)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![sides_btn].spacing(SPACING).height(Length::Fill),
        row![
            make_value_button(
                fl!("hide-time-for-last-15-seconds"),
                bool_string(*hide_time),
                (false, true),
                Some(Message::ToggleBoolParameter(BoolGameParameter::HideTime))
            ),
            make_value_button(
                fl!("player-display-brightness"),
                fl!("brightness", brightness = brightness.to_string()),
                (false, true),
                Some(Message::CycleParameter(CyclingParameter::Brightness))
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        make_cancel_apply_footer(ConfigPage::Display, settings, page_entry_snapshot),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_sound_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let EditableSettings { sound, .. } = settings;

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        row![
            make_value_button(
                fl!("sound-enabled"),
                bool_string(sound.sound_enabled),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::SoundEnabled,
                )),
            ),
            make_value_button(
                fl!("whistle-volume"),
                sound.whistle_vol.to_string(),
                (false, true),
                if sound.sound_enabled && sound.whistle_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AlertVolume))
                } else {
                    None
                },
            ),
            make_button(fl!("manage-remotes"))
                .on_press(Message::ChangeConfigPage(ConfigPage::Remotes(0, false)),)
                .style(light_gray_button),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("whistle-enabled"),
                bool_string(sound.whistle_enabled),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::RefAlertEnabled,
                    ))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("above-water-volume"),
                sound.above_water_vol.to_string(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AboveWaterVol))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("auto-sound-start-play"),
                bool_string(sound.auto_sound_start_play),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::AutoSoundStartPlay,
                    ))
                } else {
                    None
                },
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("buzzer-sound"),
                sound.buzzer_sound.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::BuzzerSound))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("underwater-volume"),
                sound.under_water_vol.to_string(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::UnderWaterVol))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("auto-sound-stop-play"),
                bool_string(sound.auto_sound_stop_play),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::AutoSoundStopPlay,
                    ))
                } else {
                    None
                },
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("alarm-button"),
                bool_string(sound.manual_alarm_enabled),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::ManualAlarmEnabled,
                    ))
                } else {
                    None
                },
            ),
            horizontal_space(),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        make_cancel_apply_footer(ConfigPage::Sound, settings, page_entry_snapshot),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

// Same situation as make_event_config_page — view builder accumulates app-state slices. Context-struct refactor filed as Findings-Backlog.
#[allow(clippy::too_many_arguments)]
fn make_remote_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    index: usize,
    listening: bool,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    const REMOTES_LIST_LEN: usize = 4;

    let title = text(fl!("remotes"))
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    let buttons: CollectArrayResult<_, REMOTES_LIST_LEN> = settings
        .sound
        .remotes
        .iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(REMOTES_LIST_LEN)
        .map(|rem| {
            if let Some((idx, rem_info)) = rem {
                let sound_text = if let Some(sound) = rem_info.sound {
                    sound.to_string().to_uppercase()
                } else {
                    fl!("default").to_owned()
                };
                let sound_text = fl!("sound", sound_text = sound_text);

                container(
                    row![
                        text(format!("ID: {}", rem_info.id))
                            .size(MEDIUM_TEXT)
                            .align_y(Vertical::Center)
                            .align_x(Horizontal::Center)
                            .height(Length::Fill)
                            .width(Length::Fill),
                        make_button(sound_text)
                            .on_press(Message::CycleParameter(
                                CyclingParameter::RemoteBuzzerSound(idx),
                            ))
                            .width(Length::Fixed(275.0))
                            .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                            .style(yellow_button),
                        make_button(fl!("delete"))
                            .on_press(Message::DeleteRemote(idx))
                            .width(Length::Fixed(130.0))
                            .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                            .style(red_button),
                    ]
                    .padding(PADDING)
                    .spacing(SPACING),
                )
                .width(Length::Fill)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(gray_container)
                .into()
            } else {
                container(horizontal_space())
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(disabled_container)
                    .into()
            }
        })
        .collect();

    let add_btn = if listening {
        make_button(fl!("waiting"))
    } else {
        make_button(fl!("add")).on_press(Message::RequestRemoteId)
    }
    .style(orange_button);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            make_scroll_list(
                buttons.unwrap(),
                settings.sound.remotes.len(),
                index,
                title,
                ScrollOption::GameParameter,
                light_gray_container,
            )
            .height(Length::Fill)
            .width(Length::FillPortion(5)),
            column![vertical_space(), add_btn,]
                .spacing(SPACING)
                .height(Length::Fill)
                .width(Length::Fill),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill),
        make_cancel_apply_footer(
            ConfigPage::Remotes(index, listening),
            settings,
            page_entry_snapshot,
        ),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

pub(in super::super) fn build_game_parameter_editor<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    length: Duration,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    let (title, hint) = match param {
        LengthParameter::Half => (
            fl!("half-length"),
            fl!("length-of-half-during-regular-play"),
        ),
        LengthParameter::HalfTime => (fl!("half-time-lenght"), fl!("length-of-half-time-period")),
        LengthParameter::NominalBetweenGame => {
            (fl!("nom-break"), fl!("system-will-keep-game-times-spaced"))
        }
        LengthParameter::MinimumBetweenGame => (fl!("min-break"), fl!("min-time-btwn-games")),
        LengthParameter::PreOvertime => (fl!("pre-ot-break-abreviated"), fl!("pre-sd-brk")),
        LengthParameter::OvertimeHalf => (fl!("ot-half-len"), fl!("time-during-ot")),
        LengthParameter::OvertimeHalfTime => {
            (fl!("ot-half-tm-len"), fl!("len-of-overtime-halftime"))
        }
        LengthParameter::PreSuddenDeath => (fl!("pre-sd-break"), fl!("pre-sd-len")),
    };

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        vertical_space(),
        make_time_editor(title, length, false),
        vertical_space(),
        text(fl!("help") + &hint)
            .size(SMALL_TEXT)
            .align_x(Horizontal::Center),
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn font_family_id(lang: Language) -> u8 {
    match lang {
        Language::Korean | Language::Japanese | Language::Mandarin => 1,
        Language::Thai => 2,
        _ => 0,
    }
}

fn make_language_select_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
) -> Element<'a, Message> {
    let selected = settings.pending_language.unwrap_or(Language::English);
    let original = settings.original_language.unwrap_or(Language::English);
    let apply_enabled = page_has_changes(ConfigPage::Language, settings, page_entry_snapshot);

    let cjk_font = iced_core::Font {
        family: iced_core::font::Family::Name("Noto Sans CJK KR"),
        weight: iced_core::font::Weight::Normal,
        stretch: iced_core::font::Stretch::Normal,
        style: iced_core::font::Style::Normal,
    };

    let thai_font = iced_core::Font {
        family: iced_core::font::Family::Name("Noto Sans Thai"),
        weight: iced_core::font::Weight::Normal,
        stretch: iced_core::font::Stretch::Normal,
        style: iced_core::font::Style::Normal,
    };

    let latin_font = iced_core::Font {
        family: iced_core::font::Family::Name("Roboto"),
        weight: iced_core::font::Weight::Medium,
        stretch: iced_core::font::Stretch::Normal,
        style: iced_core::font::Style::Normal,
    };

    // Font to apply to Cancel/Done/Restart text so they render in the target language's script
    // regardless of the app's current default font. Without an explicit Latin arm, Turkish text
    // like "İPTAL" or "BAŞLAT" renders as tofu when the app is currently in a CJK/Thai locale.
    let selected_font: Option<iced_core::Font> = match selected {
        Language::Korean | Language::Japanese | Language::Mandarin => Some(cjk_font),
        Language::Thai => Some(thai_font),
        _ => Some(latin_font),
    };

    // A restart is needed when switching between Latin and CJK font families.
    let needs_restart = font_family_id(original) != font_family_id(selected);

    let lang_btn = |lang: Language,
                    label: &'static str,
                    font: Option<iced_core::Font>|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        let label_widget = {
            let t = centered_text(label);
            if let Some(f) = font { t.font(f) } else { t }
        };
        button(label_widget)
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .style(style)
            .width(Length::Fill)
            .on_press(Message::SelectLanguage(lang))
            .into()
    };

    // Button variant for unverified translations: shows native name plus a small
    // "(UNVERIFIED)" note in that language's own script. The note text is hardcoded
    // in each target language, not routed through fl!, because fl! always renders
    // in the operator's current locale — but each button must label itself.
    let lang_btn_note = |lang: Language,
                         main: NameLines<&'static str>,
                         note: &'static str,
                         font: Option<iced_core::Font>|
     -> Element<'a, Message> {
        let style = if lang == selected {
            blue_selected_button
        } else {
            light_gray_button
        };
        make_lang_button_with_note(main, note, font)
            .style(style)
            .width(Length::Fill)
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
    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        row![
            lang_btn_note(
                Language::Indonesian,
                NameLines::OneLineSmall("BAHASA INDONESIA"),
                "(BELUM DIVERIFIKASI)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Malay,
                NameLines::OneLineSmall("BAHASA MELAYU"),
                "(BELUM DISAHKAN)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::German,
                NameLines::OneLine("DEUTSCH"),
                "(NICHT VERIFIZIERT)",
                Some(latin_font),
            ),
            lang_btn(Language::English, "ENGLISH", None),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            lang_btn(Language::Spanish, "ESPAÑOL", None),
            lang_btn_note(
                Language::Tagalog,
                NameLines::OneLine("FILIPINO"),
                "(HINDI PA NA-VERIFY)",
                Some(latin_font),
            ),
            lang_btn(Language::French, "FRANÇAIS", None),
            lang_btn_note(
                Language::Korean,
                NameLines::OneLine("한국어"),
                "(검증되지 않음)",
                Some(cjk_font),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            lang_btn_note(
                Language::Italian,
                NameLines::OneLine("ITALIANO"),
                "(NON VERIFICATO)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Dutch,
                NameLines::OneLine("NEDERLANDS"),
                "(NIET GEVERIFIEERD)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Japanese,
                NameLines::OneLine("日本語"),
                "(未検証)",
                Some(cjk_font),
            ),
            lang_btn_note(
                Language::Portuguese,
                NameLines::OneLine("PORTUGUÊS"),
                "(NÃO VERIFICADO)",
                Some(latin_font),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            lang_btn_note(
                Language::Thai,
                NameLines::OneLine("ภาษาไทย"),
                "(ยังไม่ได้ตรวจสอบ)",
                Some(thai_font),
            ),
            lang_btn_note(
                Language::Turkish,
                NameLines::OneLine("TÜRKÇE"),
                "(DOĞRULANMAMIŞ)",
                Some(latin_font),
            ),
            lang_btn_note(
                Language::Mandarin,
                NameLines::OneLine("中文"),
                "(未验证)",
                Some(cjk_font),
            ),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        {
            // Use align_x(Left) + width(Shrink) + outer container centering for all
            // dynamic text in these buttons. This ensures iced's damage tracking
            // region starts from the text's left edge, so old glyph pixels are fully
            // cleared when content changes on language switch.
            let make_label = |content: &'static str, font: Option<iced_core::Font>| {
                let t = text(content)
                    .align_x(Horizontal::Left)
                    .align_y(Vertical::Top)
                    .width(Length::Shrink);
                let t: iced::widget::Text<'a, _, _> =
                    if let Some(f) = font { t.font(f) } else { t };
                container(t).center(Length::Fill)
            };

            let cancel_btn = button(make_label(selected.cancel_text(), selected_font))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::LanguageSelectComplete { canceled: true });

            let confirm_msg =
                apply_enabled.then_some(Message::LanguageSelectComplete { canceled: false });
            let confirm_btn: Element<'a, Message> = if needs_restart {
                button(make_label(selected.restart_text(), selected_font))
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(blue_button)
                    .width(Length::Fill)
                    .on_press_maybe(confirm_msg)
                    .into()
            } else {
                button(make_label(selected.done_text(), selected_font))
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press_maybe(confirm_msg)
                    .into()
            };

            row![cancel_btn, horizontal_space(), confirm_btn]
        }
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PageEntrySnapshot;
    use crate::config::Mode;
    use matrix_drawing::transmitted_data::Brightness;
    use time::macros::datetime;
    use uwh_common::uwhportal::schedule::{Game, ScheduledTeam, TeamId};

    fn make_schedule_with_one_game(event_id: EventId, game_number: &str, court: &str) -> Schedule {
        let game = Game {
            number: game_number.to_string(),
            dark: ScheduledTeam::new_team_id(TeamId::from_partial("dark")),
            light: ScheduledTeam::new_team_id(TeamId::from_partial("light")),
            start_time: datetime!(2026-01-01 0:00 UTC),
            court: court.to_string(),
            timing_rule: "RR".to_string(),
            referee_assignments: None,
            description: None,
        };
        Schedule {
            event_id,
            games: std::iter::once((game.number.clone(), game)).collect(),
            non_game_entries: vec![],
            groups: vec![],
            timing_rules: vec![],
            standings_order: None,
            final_results_order: None,
        }
    }

    #[test]
    fn display_no_changes_when_buffer_equals_snapshot() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::Medium,
            hide_time: false,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            hide_time: false,
        };
        assert!(!page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn display_detects_brightness_change() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::High,
            hide_time: false,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            hide_time: false,
        };
        assert!(page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn page_without_snapshot_reports_no_changes() {
        let edited = EditableSettings::default();
        assert!(!page_has_changes(ConfigPage::Display, &edited, None));
    }

    // ---------------------------------------------------------------------
    // Invariant 1: per-page snapshot capture-and-revert (B3.10, B3.33)
    //
    // The Game-slice snapshot must restore every Game-slice field on Cancel,
    // while leaving fields owned by other pages alone.
    // ---------------------------------------------------------------------

    #[test]
    fn game_snapshot_revert_restores_all_game_slice_fields() {
        let event_id = EventId::from_partial("evt-A");
        let original_config = GameConfig::default();
        let mut bumped_config = GameConfig::default();
        bumped_config.team_timeout_duration += Duration::from_secs(15);

        // Entry-time state: snapshot captures this.
        let mut edited = EditableSettings {
            config: original_config.clone(),
            game_number: "1".to_string(),
            using_uwhportal: true,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id.clone(), "1", "CourtA")),
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Game {
            config: edited.config.clone(),
            game_number: edited.game_number.clone(),
            using_uwhportal: edited.using_uwhportal,
            current_event_id: edited.current_event_id.clone(),
            current_court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
        };

        // Operator mutates every Game-slice field after entering Game Options.
        edited.config = bumped_config;
        edited.game_number = "99".to_string();
        edited.using_uwhportal = false;
        edited.current_event_id = Some(EventId::from_partial("evt-B"));
        edited.current_court = Some("CourtB".to_string());
        edited.schedule = None;

        snap.revert_into(&mut edited);

        assert_eq!(edited.config, original_config);
        assert_eq!(edited.game_number, "1");
        assert!(edited.using_uwhportal);
        assert_eq!(edited.current_event_id, Some(event_id.clone()));
        assert_eq!(edited.current_court.as_deref(), Some("CourtA"));
        assert!(edited.schedule.is_some());
        assert_eq!(edited.schedule.as_ref().unwrap().event_id, event_id,);
    }

    #[test]
    fn game_snapshot_revert_leaves_other_page_slices_untouched() {
        // Entry-time Game-slice values get captured.
        let mut edited = EditableSettings {
            game_number: "1".to_string(),
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Game {
            config: edited.config.clone(),
            game_number: edited.game_number.clone(),
            using_uwhportal: edited.using_uwhportal,
            current_event_id: edited.current_event_id.clone(),
            current_court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
        };

        // Operator edits non-Game-slice fields between entering and cancelling
        // Game Options: those belong to other pages and must NOT be reverted.
        edited.mode = Mode::Rugby;
        edited.confirm_score = true;
        edited.track_fouls_and_warnings = true;
        edited.collect_scorer_cap_num = true;
        edited.white_on_right = true;
        edited.brightness = Brightness::High;
        edited.hide_time = true;

        // Also mutate a Game-slice field so we can prove the Game-slice revert
        // still happened on this same call.
        edited.game_number = "99".to_string();

        snap.revert_into(&mut edited);

        // Game-slice field was reverted.
        assert_eq!(edited.game_number, "1");

        // Other-page-slice fields are untouched.
        assert_eq!(edited.mode, Mode::Rugby);
        assert!(edited.confirm_score);
        assert!(edited.track_fouls_and_warnings);
        assert!(edited.collect_scorer_cap_num);
        assert!(edited.white_on_right);
        assert_eq!(edited.brightness, Brightness::High);
        assert!(edited.hide_time);
    }

    #[test]
    fn app_snapshot_revert_restores_only_app_slice_fields() {
        // Per ADR 009 the App page owns the portal trio plus the four App-slice
        // booleans. This test mirrors Invariant 1's assertions for App.
        let original_event = EventId::from_partial("evt-A");

        let mut edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: Some(original_event.clone()),
            current_court: Some("CourtA".to_string()),
            mode: Mode::Hockey6V6,
            collect_scorer_cap_num: false,
            track_fouls_and_warnings: false,
            confirm_score: false,
            // A Game-slice field we'll mutate to prove App revert ignores it.
            game_number: "1".to_string(),
            ..Default::default()
        };
        let snap = PageEntrySnapshot::App {
            using_uwhportal: edited.using_uwhportal,
            current_event_id: edited.current_event_id.clone(),
            current_court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
            mode: edited.mode,
            collect_scorer_cap_num: edited.collect_scorer_cap_num,
            track_fouls_and_warnings: edited.track_fouls_and_warnings,
            confirm_score: edited.confirm_score,
        };

        edited.using_uwhportal = false;
        edited.current_event_id = Some(EventId::from_partial("evt-B"));
        edited.current_court = Some("CourtB".to_string());
        edited.mode = Mode::Rugby;
        edited.collect_scorer_cap_num = true;
        edited.track_fouls_and_warnings = true;
        edited.confirm_score = true;
        edited.game_number = "99".to_string();

        snap.revert_into(&mut edited);

        // App-slice fields restored.
        assert!(edited.using_uwhportal);
        assert_eq!(edited.current_event_id, Some(original_event));
        assert_eq!(edited.current_court.as_deref(), Some("CourtA"));
        assert_eq!(edited.mode, Mode::Hockey6V6);
        assert!(!edited.collect_scorer_cap_num);
        assert!(!edited.track_fouls_and_warnings);
        assert!(!edited.confirm_score);

        // Game-slice field NOT restored by the App snapshot.
        assert_eq!(edited.game_number, "99");
    }

    // ---------------------------------------------------------------------
    // Invariant 2: uwhportal_incomplete() Apply-disable predicate (B3.9, B3.37)
    //
    // The same helper backs both the Apply-button enable state in the footer
    // and the gate check at the top of apply_game_options. The two consumers
    // must stay in sync because uwhportal_incomplete() is the only source of
    // truth — these tests lock its branches.
    // ---------------------------------------------------------------------

    #[test]
    fn uwhportal_incomplete_false_when_portal_off() {
        let edited = EditableSettings {
            using_uwhportal: false,
            current_event_id: None,
            current_court: None,
            schedule: None,
            ..Default::default()
        };
        assert!(!edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_event_missing() {
        let edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: None,
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(
                EventId::from_partial("evt-A"),
                "1",
                "CourtA",
            )),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_court_missing() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: Some(event_id.clone()),
            current_court: None,
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_schedule_missing() {
        let edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: Some(EventId::from_partial("evt-A")),
            current_court: Some("CourtA".to_string()),
            schedule: None,
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_game_not_in_schedule() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "does-not-exist".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_game_court_mismatches_current_court() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtB".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_false_when_all_present_and_matching() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            using_uwhportal: true,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(!edited.uwhportal_incomplete());
    }

    // ---------------------------------------------------------------------
    // Invariant 4: picker-driven field clearing on event/court change
    // (B3.15, B3.16)
    //
    // select_event/select_court are the helpers used by the
    // Message::ParameterSelected handler. Locking them in tests preserves the
    // documented behaviour that switching events clears court / game number /
    // schedule, and switching courts clears game number.
    // ---------------------------------------------------------------------

    #[test]
    fn select_event_sets_event_and_clears_court_game_schedule() {
        let event_id = EventId::from_partial("evt-A");
        let mut edited = EditableSettings {
            current_event_id: Some(EventId::from_partial("old-evt")),
            current_court: Some("OldCourt".to_string()),
            game_number: "42".to_string(),
            schedule: Some(make_schedule_with_one_game(
                EventId::from_partial("old-evt"),
                "42",
                "OldCourt",
            )),
            ..Default::default()
        };

        edited.select_event(event_id.clone());

        assert_eq!(edited.current_event_id, Some(event_id));
        assert_eq!(edited.current_court, None);
        assert_eq!(edited.game_number, "");
        assert!(edited.schedule.is_none());
    }

    #[test]
    fn select_court_sets_court_and_clears_game_number() {
        let event_id = EventId::from_partial("evt-A");
        let mut edited = EditableSettings {
            current_event_id: Some(event_id.clone()),
            current_court: Some("OldCourt".to_string()),
            game_number: "42".to_string(),
            schedule: Some(make_schedule_with_one_game(event_id, "42", "OldCourt")),
            ..Default::default()
        };

        edited.select_court("NewCourt".to_string());

        assert_eq!(edited.current_court.as_deref(), Some("NewCourt"));
        assert_eq!(edited.game_number, "");
        // Event id and schedule are NOT touched by a court change.
        assert!(edited.current_event_id.is_some());
        assert!(edited.schedule.is_some());
    }

    // ---------------------------------------------------------------------
    // Regression: Sound Options Apply gate after returning from Manage
    // Remotes (Unit 3 audit, S3.15 manual walkthrough, 2026-05-13).
    //
    // Previously, taking the Cancel or Apply path on the Remotes sub-page
    // consumed/cleared the page entry snapshot and never re-captured it
    // for the parent Sound page. With no snapshot, page_has_changes
    // returned false even after real sound edits, so the Sound Apply
    // button stayed permanently disabled.
    //
    // The fix re-captures the parent's snapshot inside navigate_to_parent.
    // This test documents the predicate's expected behaviour at the
    // snapshot level: with a Sound snapshot present the predicate must
    // correctly detect (or not detect) edits, and with no snapshot it
    // conservatively reports no changes (which is what disables Apply —
    // the very bug the fix prevents from occurring on the Sound page).
    // ---------------------------------------------------------------------
    #[test]
    fn sound_apply_requires_snapshot_present() {
        let mut edited = EditableSettings::default();
        let snap = PageEntrySnapshot::Sound {
            sound: edited.sound.clone(),
        };

        // 1. No edits yet -> Apply must stay disabled.
        assert!(!page_has_changes(ConfigPage::Sound, &edited, Some(&snap)));

        // 2. Operator toggles a sound field -> Apply must enable.
        edited.sound.sound_enabled ^= true;
        assert!(page_has_changes(ConfigPage::Sound, &edited, Some(&snap)));

        // 3. If the snapshot is missing (the pre-fix bug condition after
        //    returning from Manage Remotes), the predicate reports no
        //    changes regardless of edits, which leaves Apply disabled.
        //    The fix ensures this branch is not reached on Sound after a
        //    sub-page navigation; this assertion documents the predicate's
        //    conservative behaviour under None.
        assert!(!page_has_changes(ConfigPage::Sound, &edited, None));
    }
}
