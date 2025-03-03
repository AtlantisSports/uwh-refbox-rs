use super::{
    message::*,
    shared_elements::*,
    style::{
        ButtonStyle, ContainerStyle, Element, LINE_HEIGHT, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING,
        SMALL_PLUS_TEXT, SMALL_TEXT, SPACING,
    },
};
use crate::config::Mode;
use crate::sound_controller::*;
use collect_array::CollectArrayResult;
use iced::{
    Alignment, Length,
    alignment::{Horizontal, Vertical},
    widget::{TextInput, button, column, container, horizontal_space, row, text, vertical_space},
};
use matrix_drawing::transmitted_data::Brightness;
use std::collections::BTreeMap;
use tokio::time::Duration;
use uwh_common::{
    config::Game as GameConfig, game_snapshot::GameSnapshot, uwhportal::TokenValidity, uwhscores::*,
};

const NO_SELECTION_TXT: &str = "None Selected";
const LOADING_TXT: &str = "Loading...";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in super::super) struct EditableSettings {
    pub config: GameConfig,
    pub game_number: u32,
    pub white_on_right: bool,
    pub brightness: Brightness,
    pub using_uwhscores: bool,
    pub uwhscores_email: String,
    pub uwhscores_password: String,
    pub uwhportal_token: String,
    pub current_tid: Option<u32>,
    pub current_pool: Option<String>,
    pub games: Option<BTreeMap<u32, GameInfo>>,
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

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn build_game_config_edit_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
    page: ConfigPage,
    mode: Mode,
    clock_running: bool,
    uwhscores_auth: &Option<Vec<u32>>,
    uwhportal_token_valid: Option<(TokenValidity, Option<String>)>,
    touchscreen: bool,
) -> Element<'a, Message> {
    match page {
        ConfigPage::Main => make_main_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Tournament => make_tournament_config_page(
            snapshot,
            settings,
            tournaments,
            mode,
            clock_running,
            uwhscores_auth,
            uwhportal_token_valid,
            touchscreen,
        ),
        ConfigPage::Sound => make_sound_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::Display => make_display_config_page(snapshot, settings, mode, clock_running),
        ConfigPage::App => make_app_config_page(mode, snapshot, settings, clock_running),
        ConfigPage::Credentials => {
            make_credential_config_page(snapshot, settings, mode, clock_running)
        }
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
            if let Some(games) = games {
                match games.get(game_number) {
                    Some(game) => {
                        if game.pool == *cur_pool {
                            game_string_short(game)
                        } else {
                            game_large_text = false;
                            NO_SELECTION_TXT.to_string()
                        }
                    }
                    None => {
                        game_large_text = false;
                        NO_SELECTION_TXT.to_string()
                    }
                }
            } else {
                LOADING_TXT.to_string()
            }
        } else {
            String::new()
        }
    } else {
        game_number.to_string()
    };

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running,),
        make_value_button("GAME:", game_label, (true, game_large_text), game_btn_msg,),
        row![
            make_message_button(
                "TOURNAMENT OPTIONS",
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
        vertical_space(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ConfigEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button("DONE")
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

#[allow(clippy::too_many_arguments)]
fn make_tournament_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
    mode: Mode,
    clock_running: bool,
    uwhscores_auth: &Option<Vec<u32>>,
    uwhportal_token_valid: Option<(TokenValidity, Option<String>)>,
    touchscreen: bool,
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
        let tournament_label = if let Some(tournaments) = tournaments {
            if let Some(tid) = current_tid {
                match tournaments.get(tid) {
                    Some(t) => t.name.clone(),
                    None => NO_SELECTION_TXT.to_string(),
                }
            } else {
                NO_SELECTION_TXT.to_string()
            }
        } else {
            LOADING_TXT.to_string()
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
                if let Some(pool) = current_pool {
                    pool.clone()
                } else {
                    NO_SELECTION_TXT.to_string()
                }
            } else {
                LOADING_TXT.to_string()
            }
        } else {
            String::new()
        };

        let pool_btn_msg = tournaments
            .as_ref()
            .and_then(|tourns| tourns.get(&(*current_tid)?)?.pools.as_ref())
            .map(|_| Message::SelectParameter(ListableParameter::Pool));

        let auth_container = |auth| {
            let txt = match auth {
                Some(true) => "OK",
                Some(false) => "FAILED",
                None => "CHECKING...",
            };
            let style = match auth {
                Some(true) => ContainerStyle::Green,
                Some(false) => ContainerStyle::Red,
                None => ContainerStyle::Gray,
            };
            container(txt)
                .center_x()
                .center_y()
                .width(Length::Fill)
                .height(Length::Fill)
                .style(style)
        };

        let uwhscores_auth_text = column![
            text("UWHSCORES")
                .size(SMALL_PLUS_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Right)
                .width(Length::Fill),
            text("LOGIN")
                .size(SMALL_PLUS_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Right)
                .width(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let uwhportal_auth_text = column![
            text("UWHPORTAL")
                .size(SMALL_PLUS_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Right)
                .width(Length::Fill),
            text("TOKEN")
                .size(SMALL_PLUS_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Right)
                .width(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let uwhscores_auth = uwhscores_auth.as_ref().and_then(|auth| {
            if auth.is_empty() {
                Some(false)
            } else {
                Some(auth.contains(current_tid.as_ref()?))
            }
        });

        let uwhportal_auth = if matches!(uwhportal_token_valid, Some((TokenValidity::Invalid, _))) {
            Some(false)
        } else {
            None
        };

        let auth_btn_msg = if touchscreen {
            Message::NoAction
        } else {
            Message::ChangeConfigPage(ConfigPage::Credentials)
        };

        let auth_state_button = button(
            row![
                uwhscores_auth_text,
                auth_container(uwhscores_auth),
                uwhportal_auth_text,
                auth_container(uwhportal_auth),
            ]
            .padding(PADDING)
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .padding(0)
        .style(ButtonStyle::LightGray)
        .on_press(auth_btn_msg);

        [
            make_value_button(
                "TOURNAMENT:",
                tournament_label,
                (true, true),
                tournament_btn_msg,
            )
            .height(Length::Fill)
            .into(),
            make_value_button("COURT:", pool_label, (true, true), pool_btn_msg)
                .height(Length::Fill)
                .into(),
            auth_state_button.into(),
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
            .into(),
        ]
    } else {
        [
            row![
                make_value_button(
                    "HALF LENGTH:",
                    time_string(config.half_play_duration),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::Half)),
                ),
                make_value_button(
                    "OVERTIME\nALLOWED:",
                    bool_string(config.overtime_allowed),
                    (false, true),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::OvertimeAllowed,
                    )),
                ),
                make_value_button(
                    "SUDDEN DEATH\nALLOWED:",
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
                    "HALF TIME\nLENGTH:",
                    time_string(config.half_time_duration),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
                ),
                make_value_button(
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    (false, true),
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::PreOvertime))
                    } else {
                        None
                    },
                ),
                make_value_button(
                    "PRE SD\nBREAK LENGTH:",
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
                    "NOMINAL BRK\nBTWN GAMES:",
                    time_string(config.nominal_break),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                ),
                make_value_button(
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    (false, true),
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                    } else {
                        None
                    },
                ),
                make_value_button(
                    "NUM TEAM T/Os\nALLOWED:",
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
                    "MINIMUM BRK\nBTWN GAMES:",
                    time_string(config.minimum_break),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                ),
                make_value_button(
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    (false, true),
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
                    } else {
                        None
                    },
                ),
                make_button("DONE")
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
            "USING UWHPORTAL:",
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
        confirm_score,
        ..
    } = settings;

    column![
        make_game_time_button(snapshot, false, true, mode, clock_running),
        row![
            make_value_button(
                "APP\nMODE",
                settings.mode.to_string().to_uppercase(),
                (false, true),
                Some(Message::CycleParameter(CyclingParameter::Mode)),
            ),
            make_value_button(
                "TRACK CAP NUMBER\nOF SCORER",
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
            make_value_button(
                "CONFIRM SCORE\nAT GAME END",
                bool_string(*confirm_score),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ConfirmScore,
                )),
            ),
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
        brightness,
        ..
    } = settings;

    let white = container("WHITE")
        .center_x()
        .center_y()
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(ContainerStyle::White);
    let black = container("BLACK")
        .center_x()
        .center_y()
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(ContainerStyle::Black);

    let center = text("STARTING SIDES")
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
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .padding(0)
        .style(ButtonStyle::LightGray)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![sides_btn].spacing(SPACING),
        row![
            make_value_button(
                "HIDE TIME FOR\nLAST 15 SECONDS",
                bool_string(*hide_time),
                (false, true),
                Some(Message::ToggleBoolParameter(BoolGameParameter::HideTime))
            ),
            make_value_button(
                "PLAYER DISPLAY\nBRIGHTNESS",
                brightness.to_string().to_uppercase(),
                (false, true),
                Some(Message::CycleParameter(CyclingParameter::Brightness))
            )
        ]
        .spacing(SPACING),
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
                "SOUND\nENABLED:",
                bool_string(sound.sound_enabled),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::SoundEnabled,
                )),
            ),
            make_value_button(
                "WHISTLE\nVOLUME:",
                sound.whistle_vol.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled && sound.whistle_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AlertVolume))
                } else {
                    None
                },
            ),
            make_message_button(
                "MANAGE REMOTES",
                Some(Message::ChangeConfigPage(ConfigPage::Remotes(0, false))),
            )
            .style(ButtonStyle::LightGray),
        ]
        .spacing(SPACING),
        row![
            make_value_button(
                "WHISTLE\nENABLED:",
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
                "ABOVE WATER\nVOLUME:",
                sound.above_water_vol.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AboveWaterVol))
                } else {
                    None
                },
            ),
            make_value_button(
                "AUTO SOUND\nSTART PLAY:",
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
                "BUZZER\nSOUND:",
                sound.buzzer_sound.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::BuzzerSound))
                } else {
                    None
                },
            ),
            make_value_button(
                "UNDER WATER\nVOLUME:",
                sound.under_water_vol.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::UnderWaterVol))
                } else {
                    None
                },
            ),
            make_value_button(
                "AUTO SOUND\nSTOP PLAY:",
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
        vertical_space(Length::Fill),
        row![
            horizontal_space(Length::Fill),
            horizontal_space(Length::Fill),
            make_button("DONE")
                .style(ButtonStyle::Green)
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

    let title = text("REMOTES")
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
                    "DEFAULT".to_owned()
                };
                let sound_text = format!("SOUND: {}", sound_text);

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
                        make_message_button("DELETE", Some(Message::DeleteRemote(idx)))
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
        make_message_button("WAITING", None)
    } else {
        make_message_button("ADD", Some(Message::RequestRemoteId))
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
                make_message_button("DONE", Some(Message::ChangeConfigPage(ConfigPage::Sound)),)
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

fn make_credential_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
) -> Element<'a, Message> {
    let EditableSettings {
        uwhscores_email,
        uwhscores_password,
        uwhportal_token,
        ..
    } = settings;

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        row![
            text("UWHSCORES EMAIL:")
                .size(MEDIUM_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .height(Length::Fill),
            TextInput::new("", uwhscores_email,)
                .on_input(|s| Message::TextParameterChanged(TextParameter::UwhscoresEmail, s))
                .width(Length::Fill)
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            text("UWHSCORES PASSWORD:")
                .size(MEDIUM_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .height(Length::Fill),
            TextInput::new("", uwhscores_password,)
                .on_input(|s| Message::TextParameterChanged(TextParameter::UwhscoresPassword, s))
                .password()
                .width(Length::Fill)
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            text("UWHPORTAL TOKEN:")
                .size(MEDIUM_TEXT)
                .line_height(LINE_HEIGHT)
                .vertical_alignment(Vertical::Center)
                .height(Length::Fill),
            TextInput::new("", uwhportal_token,)
                .on_input(|s| Message::TextParameterChanged(TextParameter::UwhportalToken, s))
                .password()
                .width(Length::Fill)
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        vertical_space(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ChangeConfigPage(ConfigPage::Tournament)),
            horizontal_space(Length::Fill),
            make_button("DONE")
                .style(ButtonStyle::Green)
                .width(Length::Fill)
                .on_press(Message::ApplyAuthChanges),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
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
        LengthParameter::Half => ("HALF LEN", "The length of a half during regular play"),
        LengthParameter::HalfTime => ("HALF TIME LEN", "The length of the Half Time period"),
        LengthParameter::NominalBetweenGame => (
            "NOM BREAK",
            "The system will try to keep the game start times evenly spaced, with the \
            total time from one start to the next being 2 * [Half Length] + [Half Time \
            Length] + [Nominal Time Between Games] (example: if games have [Half \
            Length] = 15m, [Half Time Length] = 3m, and [Nominal Time Between Games] = \
            12m, the time from the start of one game to the next will be 45m. Any \
            timeouts taken, or other clock stoppages, will reduce the 12m time down \
            until the minimum time between game value is reached).",
        ),
        LengthParameter::MinimumBetweenGame => (
            "MIN BREAK",
            "If a game runs longer than scheduled, this is the minimum time between \
            games that the system will allot. If the games fall behind, the system will \
            automatically try to catch up after subsequent games, always respecting \
            this minimum time between games.",
        ),
        LengthParameter::PreOvertime => (
            "PRE OT BREAK",
            "If overtime is enabled and needed, this is the length of the break between \
            Second Half and Overtime First Half",
        ),
        LengthParameter::OvertimeHalf => ("OT HALF LEN", "The length of a half during overtime"),
        LengthParameter::OvertimeHalfTime => ("OT HLF TM LEN", "The length of Overtime Half Time"),
        LengthParameter::PreSuddenDeath => (
            "PRE SD BREAK",
            "The length of the break between the preceeding play period and Sudden Death",
        ),
    };

    column![
        make_game_time_button(snapshot, false, false, mode, clock_running),
        vertical_space(Length::Fill),
        make_time_editor(title, length, false),
        vertical_space(Length::Fill),
        text(String::from("Help: ") + hint)
            .size(SMALL_TEXT)
            .line_height(LINE_HEIGHT)
            .horizontal_alignment(Horizontal::Center),
        vertical_space(Length::Fill),
        row![
            make_button("CANCEL")
                .style(ButtonStyle::Red)
                .width(Length::Fill)
                .on_press(Message::ParameterEditComplete { canceled: true }),
            horizontal_space(Length::Fill),
            make_button("DONE")
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
