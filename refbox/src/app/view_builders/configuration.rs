use super::{
    fl,
    message::*,
    shared_elements::*,
    style::{
        ButtonStyle, ContainerStyle, Element, LINE_HEIGHT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING,
        SMALL_TEXT, SPACING,
    },
};
use crate::config::Mode;
use crate::sound_controller::*;
use collect_array::CollectArrayResult;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{button, column, container, horizontal_space, row, text, vertical_space},
    Alignment, Length,
};
use std::collections::BTreeMap;
use tokio::time::Duration;
use uwh_common::{config::Game as GameConfig, game_snapshot::GameSnapshot, uwhscores::*};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in super::super) struct EditableSettings {
    pub config: GameConfig,
    pub game_number: u32,
    pub white_on_right: bool,
    pub using_uwhscores: bool,
    pub current_tid: Option<u32>,
    pub current_pool: Option<String>,
    pub games: Option<BTreeMap<u32, GameInfo>>,
    pub sound: SoundSettings,
    pub mode: Mode,
    pub hide_time: bool,
    pub collect_scorer_cap_num: bool,
    pub track_fouls_and_warnings: bool,
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

pub(in super::super) fn build_game_config_edit_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
    page: ConfigPage,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    match page {
        ConfigPage::Main => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Tournament => {
            make_tournament_config_page(snapshot, settings, tournaments, mode, clock_running)
        }
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
        using_uwhscores,
        current_tid,
        current_pool,
        games,
        ..
    } = settings;

    let using_uwhscores = *using_uwhscores;

    let game_btn_msg = if using_uwhscores {
        if current_tid.is_some() && current_pool.is_some() {
            Some(Message::SelectParameter(ListableParameter::Game))
        } else {
            None
        }
    } else {
        Some(Message::KeypadPage(KeypadPage::GameNumber))
    };

    let mut game_large_text = true;
    let game_label = if using_uwhscores {
        if let (Some(_), Some(cur_pool)) = (current_tid, current_pool) {
            if let Some(ref games) = games {
                match games.get(game_number) {
                    Some(game) => {
                        if game.pool == *cur_pool {
                            game_string_short(game)
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
        make_game_time_button(snapshot, false, false, mode, clock_running,),
        make_value_button(
            fl!("game"),
            game_label,
            (true, game_large_text),
            game_btn_msg,
        ),
        row![
            make_message_button(
                fl!("tournament-options"),
                Some(Message::ChangeConfigPage(ConfigPage::Tournament)),
            )
            .style(ButtonStyle::LightGray),
            make_message_button(
                "APP OPTIONS",
                Some(Message::ChangeConfigPage(ConfigPage::App)),
            )
            .style(ButtonStyle::LightGray),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
        row![
            make_message_button(
                "DISPLAY OPTIONS",
                Some(Message::ChangeConfigPage(ConfigPage::Display)),
            )
            .style(ButtonStyle::LightGray),
            make_message_button(
                "SOUND OPTIONS",
                Some(Message::ChangeConfigPage(ConfigPage::Sound)),
            )
            .style(ButtonStyle::LightGray),
        ]
        .spacing(SPACING)
        .width(Length::Fill)
        .height(Length::Fill),
        row![
            make_button(fl!("cancel"))
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button(fl!("done"))
                .style(ButtonStyle::Green)
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

fn make_tournament_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let EditableSettings {
        config,
        using_uwhscores,
        current_tid,
        current_pool,
        ..
    } = settings;

    let using_uwhscores = *using_uwhscores;

    let rows: [Element<Message>; 4] = if using_uwhscores {
        let tournament_label = if let Some(ref tournaments) = tournaments {
            if let Some(tid) = current_tid {
                match tournaments.get(tid) {
                    Some(t) => t.name.clone(),
                    None => fl!("none-selected"),
                }
            } else {
                fl!("none-selected")
            }
        } else {
            fl!("loading")
        };

        let tournament_btn_msg = if tournaments.is_some() {
            Some(Message::SelectParameter(ListableParameter::Tournament))
        } else {
            None
        };

        let pool_label = if let Some(tournament) = tournaments
            .as_ref()
            .and_then(|tournaments| tournaments.get(&(*current_tid)?))
        {
            if tournament.pools.is_some() {
                if let Some(ref pool) = current_pool {
                    pool.clone()
                } else {
                    fl!("none-selected")
                }
            } else {
                fl!("loading")
            }
        } else {
            String::new()
        };

        let pool_btn_msg = tournaments
            .as_ref()
            .and_then(|tourns| tourns.get(&(*current_tid)?)?.pools.as_ref())
            .map(|_| Message::SelectParameter(ListableParameter::Pool));

        [
            make_value_button(
                fl!("tournament"),
                tournament_label,
                (true, true),
                tournament_btn_msg,
            )
            .height(Length::Fill)
            .into(),
            make_value_button(fl!("court"), pool_label, (true, true), pool_btn_msg)
                .height(Length::Fill)
                .into(),
            vertical_space(Length::Fill).into(),
            row![
                horizontal_space(Length::Fill),
                horizontal_space(Length::Fill),
                make_button(fl!("done"))
                    .style(ButtonStyle::Green)
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
                    fl!("half-length"),
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
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
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
                    fl!("num-team-tos-allowed-per-half"),
                    config.team_timeouts_per_half.to_string(),
                    (false, true),
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        config.team_timeout_duration,
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
                    .style(ButtonStyle::Green)
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
        make_value_button(
            fl!("using-uwh-portal"),
            bool_string(using_uwhscores),
            (true, true),
            Some(Message::ToggleBoolParameter(
                BoolGameParameter::UsingUwhScores,
            )),
        )
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
        ..
    } = settings;

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        row![
            make_value_button(
                fl!("app-mode"),
                settings.mode.to_string().to_uppercase(),
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
                "TRACK FOULS\nAND WARNINGS",
                bool_string(*track_fouls_and_warnings),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::FoulsAndWarnings,
                )),
            ),
            horizontal_space(Length::Fill),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        vertical_space(Length::Fill),
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            horizontal_space(Length::Fill),
            make_button("DONE")
                .style(ButtonStyle::Green)
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
        ..
    } = settings;

    let white = container(text(fl!("dark-team-name-caps")))
        .center_x()
        .center_y()
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(ContainerStyle::White);
    let black = container(text(fl!("light-team-name-caps")))
        .center_x()
        .center_y()
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(ContainerStyle::Black);

    let center = text(fl!("starting-sides"))
        .size(MEDIUM_TEXT)
        .line_height(LINE_HEIGHT)
        .vertical_alignment(Vertical::Center)
        .horizontal_alignment(Horizontal::Center)
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
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(0)
        .style(ButtonStyle::LightGray)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![sides_btn].spacing(SPACING).height(Length::Fill),
        row![make_value_button(
            fl!("hide-time-for-last-15-seconds"),
            bool_string(*hide_time),
            (false, true),
            Some(Message::ToggleBoolParameter(BoolGameParameter::HideTime))
        )]
        .spacing(SPACING)
        .height(Length::Fill),
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            horizontal_space(Length::Fill),
            make_button("DONE")
                .style(ButtonStyle::Green)
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
                sound.whistle_vol.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled && sound.whistle_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AlertVolume))
                } else {
                    None
                },
            ),
            make_message_button(
                fl!("manage-remotes"),
                Some(Message::ChangeConfigPage(ConfigPage::Remotes(0, false))),
            )
            .style(ButtonStyle::LightGray)
            .height(Length::Fill),
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
                sound.above_water_vol.to_string().to_uppercase(),
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
                sound.under_water_vol.to_string().to_uppercase(),
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
            horizontal_space(Length::Fill),
            horizontal_space(Length::Fill),
            make_button(fl!("done"))
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
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
        .line_height(LINE_HEIGHT)
        .height(Length::Fill)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .vertical_alignment(Vertical::Center);

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
                        text(format!("ID: {:05X}", rem_info.id))
                            .size(MEDIUM_TEXT)
                            .line_height(LINE_HEIGHT)
                            .vertical_alignment(Vertical::Center)
                            .horizontal_alignment(Horizontal::Center)
                            .height(Length::Fill)
                            .width(Length::Fill),
                        make_message_button(
                            sound_text,
                            Some(Message::CycleParameter(
                                CyclingParameter::RemoteBuzzerSound(idx),
                            )),
                        )
                        .width(Length::Fixed(275.0))
                        .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                        .style(ButtonStyle::Yellow),
                        make_message_button(fl!("delete"), Some(Message::DeleteRemote(idx)))
                            .width(Length::Fixed(130.0))
                            .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                            .style(ButtonStyle::Red),
                    ]
                    .padding(PADDING)
                    .spacing(SPACING),
                )
                .width(Length::Fill)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(ContainerStyle::Gray)
                .into()
            } else {
                container(horizontal_space(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(ContainerStyle::Disabled)
                    .into()
            }
        })
        .collect();

    let add_btn = if listening {
        make_message_button(fl!("waiting"), None)
    } else {
        make_message_button(fl!("add"), Some(Message::RequestRemoteId))
    }
    .style(ButtonStyle::Orange);

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            make_scroll_list(
                buttons.unwrap(),
                settings.sound.remotes.len(),
                index,
                title,
                ScrollOption::GameParameter,
                ContainerStyle::LightGray,
            )
            .height(Length::Fill)
            .width(Length::FillPortion(5)),
            column![
                vertical_space(Length::Fill),
                add_btn,
                make_message_button(
                    fl!("done"),
                    Some(Message::ChangeConfigPage(ConfigPage::Sound)),
                )
                .style(ButtonStyle::Green),
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
    snapshot: &GameSnapshot,
    param: LengthParameter,
    length: Duration,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
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
        LengthParameter::PreOvertime => (fl!("pre-ot-break"), fl!("pre-sd-brk")),
        LengthParameter::OvertimeHalf => (fl!("ot-half-len"), fl!("time-during-ot")),
        LengthParameter::OvertimeHalfTime => {
            (fl!("ot-half-tm-len"), fl!("len-of-overtime-halftime"))
        }
        LengthParameter::PreSuddenDeath => (fl!("pre-sd-break"), fl!("pre-sd-len")),
    };

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        vertical_space(Length::Fill),
        make_time_editor(title, length, false),
        vertical_space(Length::Fill),
        text(String::from(fl!("help")) + &hint)
            .size(SMALL_TEXT)
            .line_height(LINE_HEIGHT)
            .horizontal_alignment(Horizontal::Center),
        vertical_space(Length::Fill),
        row![
            make_button(fl!("cancel"))
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button(fl!("done"))
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: false }),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .align_items(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
