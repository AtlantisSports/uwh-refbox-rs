use super::{ViewData, fl, message::*, shared_elements::*, theme::*};
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
        ConfigPage::Remotes(index, listening) => {
            make_remote_config_page(snapshot, settings, index, listening, mode, clock_running)
        }
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
                    Some(game) => {
                        if game.court == *cur_court {
                            game.number.to_string()
                        } else {
                            game_large_text = false;
                            fl!("none-selected")
                        }
                    }
                    None => {
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
                Some(Message::CycleParameter(CyclingParameter::Language,)),
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
