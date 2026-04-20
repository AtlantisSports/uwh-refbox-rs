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

#[cfg_attr(not(test), expect(dead_code))]
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
            },
        ) => edited.config != *config || edited.game_number != *game_number,
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
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        ..
    } = data;

    match page {
        ConfigPage::Main => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Game => make_event_config_page(snapshot, settings, events, mode, clock_running),
        ConfigPage::Sound => make_sound_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Display => make_display_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::App => make_app_config_page(mode, snapshot, settings, clock_running),
        ConfigPage::User => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Remotes(index, listening) => {
            make_remote_config_page(snapshot, settings, index, listening, mode, clock_running)
        }
        ConfigPage::Language => make_language_select_page(snapshot, settings, mode, clock_running),
    }
}

fn make_main_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let EditableSettings {
        game_number,
        using_uwhportal,
        current_event_id,
        current_court,
        schedule,
        ..
    } = settings;

    let using_uwhportal = *using_uwhportal;

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

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        make_value_button(
            fl!("game-select"),
            game_label,
            (true, game_large_text),
            game_btn_msg,
        ),
        row![
            make_button(fl!("game-options"))
                .on_press(Message::ChangeConfigPage(ConfigPage::Game),)
                .style(light_gray_button),
            make_button(fl!("app-options"))
                .on_press(Message::ChangeConfigPage(ConfigPage::App),)
                .style(light_gray_button),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
        row![
            make_button(fl!("display-options"))
                .on_press(Message::ChangeConfigPage(ConfigPage::Display),)
                .style(light_gray_button),
            make_button(fl!("sound-options"))
                .on_press(Message::ChangeConfigPage(ConfigPage::Sound),)
                .style(light_gray_button),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
        vertical_space(),
        row![
            make_button(fl!("cancel"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: false }),
        ]
        .spacing(SPACING)
        .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn make_event_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    events: Option<&BTreeMap<EventId, Event>>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let EditableSettings {
        config,
        using_uwhportal,
        current_event_id,
        current_court,
        ..
    } = settings;

    let using_uwhportal = *using_uwhportal;

    let rows: [Element<Message>; 4] = if using_uwhportal {
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
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .padding(0)
        .style(light_gray_button)
        .on_press_maybe(auth_state_message);

        [
            make_value_button(fl!("event"), event_label, (true, true), event_btn_msg)
                .height(Length::Fill)
                .into(),
            auth_state_button.into(),
            make_value_button(fl!("court"), pool_label, (true, true), pool_btn_msg)
                .height(Length::Fill)
                .into(),
            row![
                horizontal_space(),
                horizontal_space(),
                make_button(fl!("done"))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
            ]
            .spacing(SPACING)
            .height(Length::Fill)
            .into(),
        ]
    } else {
        [
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
            .height(Length::Fill)
            .into(),
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
            .height(Length::Fill)
            .into(),
            row![
                make_value_button(
                    fl!("nominal-break-between-games"),
                    time_string(config.nominal_break),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
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
                )
            ]
            .spacing(SPACING)
            .height(Length::Fill)
            .into(),
            row![
                make_value_button(
                    fl!("minimum-brk-btwn-games"),
                    time_string(config.minimum_break),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
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
                make_button(fl!("done"))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
            ]
            .spacing(SPACING)
            .height(Length::Fill)
            .into(),
        ]
    };

    let mut col = column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            if !using_uwhportal {
                make_value_button(
                    fl!("single-half"),
                    bool_string(settings.config.single_half),
                    (false, true),
                    Some(Message::ToggleBoolParameter(BoolGameParameter::SingleHalf)),
                )
            } else {
                make_button("")
                    .style(light_gray_button)
                    .on_press(Message::NoAction)
            },
            make_button("")
                .style(light_gray_button)
                .on_press(Message::NoAction),
            make_value_button(
                fl!("using-uwh-portal"),
                bool_string(using_uwhportal),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::UsingUwhPortal,
                )),
            )
        ]
        .spacing(SPACING)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    for row in rows {
        col = col.push(row);
    }

    col.into()
}

fn make_app_config_page<'a>(
    mode: Mode,
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    clock_running: bool,
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
        row![
            make_value_button(
                fl!("language"),
                fl!("this-language"),
                (false, true),
                Some(Message::ChangeConfigPage(ConfigPage::Language)),
            ),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        vertical_space(),
        row![
            horizontal_space(),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
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
        row![sides_btn].spacing(SPACING),
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
        .spacing(SPACING),
        vertical_space(),
        row![
            horizontal_space(),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
        ]
        .spacing(SPACING)
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
        .spacing(SPACING),
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
        .spacing(SPACING),
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
        .spacing(SPACING),
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
        .spacing(SPACING),
        vertical_space(),
        row![
            horizontal_space(),
            horizontal_space(),
            make_button(fl!("done"))
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_remote_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    index: usize,
    listening: bool,
    mode: Mode,
    clock_running: bool,
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
            column![
                vertical_space(),
                add_btn,
                make_button(fl!("done"))
                    .on_press(Message::ChangeConfigPage(ConfigPage::Sound),)
                    .style(green_button),
            ]
            .spacing(SPACING)
            .height(Length::Fill)
            .width(Length::Fill),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill),
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
) -> Element<'a, Message> {
    let selected = settings.pending_language.unwrap_or(Language::English);
    let original = settings.original_language.unwrap_or(Language::English);

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

            let confirm_btn: Element<'a, Message> = if needs_restart {
                button(make_label(selected.restart_text(), selected_font))
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(blue_button)
                    .width(Length::Fill)
                    .on_press(Message::LanguageSelectComplete { canceled: false })
                    .into()
            } else {
                button(make_label(selected.done_text(), selected_font))
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press(Message::LanguageSelectComplete { canceled: false })
                    .into()
            };

            row![cancel_btn, horizontal_space(), confirm_btn]
        }
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PageEntrySnapshot;
    use matrix_drawing::transmitted_data::Brightness;

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
}
