use super::{
    message::*,
    shared_elements::*,
    style::{self, MEDIUM_TEXT, MIN_BUTTON_SIZE, PADDING, SMALL_TEXT, SPACING},
};
use crate::sound_controller::*;
use collect_array::CollectArrayResult;
use iced::{
    alignment::{Horizontal, Vertical},
    pure::{button, column, container, horizontal_space, row, text, vertical_space, Element},
    Alignment, Length,
};
use std::collections::BTreeMap;
use tokio::time::Duration;
use uwh_common::{config::Game as GameConfig, game_snapshot::GameSnapshot, uwhscores::*};

const NO_SELECTION_TXT: &str = "None Selected";
const LOADING_TXT: &str = "Loading...";

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

pub(in super::super) fn build_game_config_edit_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
    page: ConfigPage,
) -> Element<'a, Message> {
    match page {
        ConfigPage::Main => make_main_config_page(snapshot, settings),
        ConfigPage::Tournament => make_tournament_config_page(snapshot, settings, tournaments),
        ConfigPage::Sound => make_sound_config_page(snapshot, settings),
        ConfigPage::Remotes(index, listening) => {
            make_remote_config_page(snapshot, settings, index, listening)
        }
    }
}

fn make_main_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
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

    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(make_value_button(
            "GAME:",
            game_label,
            (true, game_large_text),
            game_btn_msg,
        ))
        .push(
            make_message_button(
                "TOURNAMENT OPTIONS",
                Some(Message::ChangeConfigPage(ConfigPage::Tournament)),
            )
            .style(style::Button::LightGray),
        )
        .push(
            make_message_button(
                "POOL AND SOUND OPTIONS",
                Some(Message::ChangeConfigPage(ConfigPage::Sound)),
            )
            .style(style::Button::LightGray),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .width(Length::Fill)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ConfigEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ConfigEditComplete { canceled: false }),
                ),
        )
        .into()
}

fn make_tournament_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    tournaments: &Option<BTreeMap<u32, TournamentInfo>>,
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
                if let Some(ref pool) = current_pool {
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
            vertical_space(Length::Fill).into(),
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(horizontal_space(Length::Fill))
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
                )
                .into(),
        ]
    } else {
        [
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "HALF LENGTH:",
                    time_string(config.half_play_duration),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::Half)),
                ))
                .push(make_value_button(
                    "OVERTIME\nALLOWED:",
                    bool_string(config.overtime_allowed),
                    (false, true),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::OvertimeAllowed,
                    )),
                ))
                .push(make_value_button(
                    "SUDDEN DEATH\nALLOWED:",
                    bool_string(config.sudden_death_allowed),
                    (false, true),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::SuddenDeathAllowed,
                    )),
                ))
                .into(),
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "HALF TIME\nLENGTH:",
                    time_string(config.half_time_duration),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::HalfTime)),
                ))
                .push(make_value_button(
                    "PRE OT\nBREAK LENGTH:",
                    time_string(config.pre_overtime_break),
                    (false, true),
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::PreOvertime))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "PRE SD\nBREAK LENGTH:",
                    time_string(config.pre_sudden_death_duration),
                    (false, true),
                    if config.sudden_death_allowed {
                        Some(Message::EditParameter(LengthParameter::PreSuddenDeath))
                    } else {
                        None
                    },
                ))
                .into(),
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "NOMINAL BRK\nBTWN GAMES:",
                    time_string(config.nominal_break),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::NominalBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nLENGTH:",
                    time_string(config.ot_half_play_duration),
                    (false, true),
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "NUM TEAM T/Os\nALLWD PER HALF:",
                    config.team_timeouts_per_half.to_string(),
                    (false, true),
                    Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                        config.team_timeout_duration,
                    ))),
                ))
                .into(),
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "MINIMUM BRK\nBTWN GAMES:",
                    time_string(config.minimum_break),
                    (false, true),
                    Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                ))
                .push(make_value_button(
                    "OT HALF\nTIME LENGTH:",
                    time_string(config.ot_half_time_duration),
                    (false, true),
                    if config.overtime_allowed {
                        Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
                    } else {
                        None
                    },
                ))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
                )
                .into(),
        ]
    };

    let mut col = column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(
            make_value_button(
                "USING UWHSCORES:",
                bool_string(using_uwhscores),
                (true, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::UsingUwhScores,
                )),
            )
            .height(Length::Fill),
        );

    for row in rows {
        col = col.push(row);
    }

    col.into()
}

fn make_sound_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
) -> Element<'a, Message> {
    let EditableSettings {
        white_on_right,
        sound,
        ..
    } = settings;

    let white = container("WHITE")
        .center_x()
        .center_y()
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(style::Container::White);
    let black = container("BLACK")
        .center_x()
        .center_y()
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(style::Container::Black);

    let center = text("STARTING SDIES")
        .size(MEDIUM_TEXT)
        .vertical_alignment(Vertical::Center)
        .horizontal_alignment(Horizontal::Center)
        .width(Length::FillPortion(3));

    // `white_on_right` is based on the view from the front of the panels, so for the ref's point
    // of view we need to reverse the direction
    let sides = if *white_on_right {
        // White to Ref's right
        row().padding(PADDING).push(black).push(center).push(white)
    } else {
        // White to Ref's left
        row().padding(PADDING).push(white).push(center).push(black)
    };

    let sides_btn = button(sides.width(Length::Fill).height(Length::Fill))
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(0)
        .style(style::Button::LightGray)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(sides_btn)
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "SOUND\nENABLED:",
                    bool_string(sound.sound_enabled),
                    (false, true),
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::SoundEnabled,
                    )),
                ))
                .push(make_value_button(
                    "REF WARN\nVOLUME:",
                    sound.ref_warn_vol.to_string().to_uppercase(),
                    (false, true),
                    if sound.sound_enabled && sound.ref_warn_enabled {
                        Some(Message::CycleParameter(CyclingParameter::WarningVolume))
                    } else {
                        None
                    },
                ))
                .push(
                    make_message_button(
                        "MANAGE REMOTES",
                        Some(Message::ChangeConfigPage(ConfigPage::Remotes(0, false))),
                    )
                    .style(style::Button::LightGray)
                    .height(Length::Fill),
                ),
        )
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "REF WARN\nENABLED:",
                    bool_string(sound.ref_warn_enabled),
                    (false, true),
                    if sound.sound_enabled {
                        Some(Message::ToggleBoolParameter(
                            BoolGameParameter::RefWarnEnabled,
                        ))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "ABOVE WATER\nVOLUME:",
                    sound.above_water_vol.to_string().to_uppercase(),
                    (false, true),
                    if sound.sound_enabled {
                        Some(Message::CycleParameter(CyclingParameter::AboveWaterVol))
                    } else {
                        None
                    },
                ))
                .push(horizontal_space(Length::Fill)),
        )
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(make_value_button(
                    "BUZZER\nSOUND:",
                    sound.buzzer_sound.to_string().to_uppercase(),
                    (false, true),
                    if sound.sound_enabled {
                        Some(Message::CycleParameter(CyclingParameter::BuzzerSound))
                    } else {
                        None
                    },
                ))
                .push(make_value_button(
                    "UNDER WATER\nVOLUME:",
                    sound.under_water_vol.to_string().to_uppercase(),
                    (false, true),
                    if sound.sound_enabled {
                        Some(Message::CycleParameter(CyclingParameter::UnderWaterVol))
                    } else {
                        None
                    },
                ))
                .push(horizontal_space(Length::Fill)),
        )
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .push(horizontal_space(Length::Fill))
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ChangeConfigPage(ConfigPage::Main)),
                ),
        )
        .into()
}

fn make_remote_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    index: usize,
    listening: bool,
) -> Element<'a, Message> {
    const REMOTES_LIST_LEN: usize = 4;

    let title = text("REMOTES")
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
                    row()
                        .padding(PADDING)
                        .spacing(SPACING)
                        .push(
                            text(format!("ID: {:05X}", rem_info.id))
                                .size(MEDIUM_TEXT)
                                .vertical_alignment(Vertical::Center)
                                .horizontal_alignment(Horizontal::Center)
                                .height(Length::Fill)
                                .width(Length::Fill),
                        )
                        .push(
                            make_message_button(
                                sound_text,
                                Some(Message::CycleParameter(
                                    CyclingParameter::RemoteBuzzerSound(idx),
                                )),
                            )
                            .width(Length::Units(275))
                            .height(Length::Units(MIN_BUTTON_SIZE - (2 * PADDING)))
                            .style(style::Button::Yellow),
                        )
                        .push(
                            make_message_button("DELETE", Some(Message::DeleteRemote(idx)))
                                .width(Length::Units(130))
                                .height(Length::Units(MIN_BUTTON_SIZE - (2 * PADDING)))
                                .style(style::Button::Red),
                        ),
                )
                .width(Length::Fill)
                .height(Length::Units(MIN_BUTTON_SIZE))
                .style(style::Container::Gray)
                .into()
            } else {
                container(horizontal_space(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Units(MIN_BUTTON_SIZE))
                    .style(style::Container::Disabled)
                    .into()
            }
        })
        .collect();

    let add_btn = if listening {
        make_message_button("WAITING", None)
    } else {
        make_message_button("ADD", Some(Message::RequestRemoteId))
    }
    .style(style::Button::Orange);

    column()
        .spacing(SPACING)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(
            row()
                .spacing(SPACING)
                .height(Length::Fill)
                .width(Length::Fill)
                .push(
                    make_scroll_list(
                        buttons.unwrap(),
                        settings.sound.remotes.len(),
                        index,
                        title,
                        ScrollOption::GameParameter,
                        style::Container::LightGray,
                    )
                    .height(Length::Fill)
                    .width(Length::FillPortion(5)),
                )
                .push(
                    column()
                        .spacing(SPACING)
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .push(vertical_space(Length::Fill))
                        .push(add_btn)
                        .push(
                            make_message_button(
                                "DONE",
                                Some(Message::ChangeConfigPage(ConfigPage::Sound)),
                            )
                            .style(style::Button::Green),
                        ),
                ),
        )
        .into()
}

pub(in super::super) fn build_game_parameter_editor<'a>(
    snapshot: &GameSnapshot,
    param: LengthParameter,
    length: Duration,
) -> Element<'a, Message> {
    let (title, hint) = match param {
        LengthParameter::Half => ("HALF LEN", "The length of a half during regular play"),
        LengthParameter::HalfTime => ("HALF TIME LEN", "The length of the Half Time period"),
        LengthParameter::NominalBetweenGame => (
            "NOM BREAK",
            "If a game runs exactly as long as scheduled, this is the length of the \
            break between games",
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

    column()
        .spacing(SPACING)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(make_game_time_button(snapshot, false, true).on_press(Message::EditTime))
        .push(vertical_space(Length::Fill))
        .push(make_time_editor(title, length, false))
        .push(vertical_space(Length::Fill))
        .push(
            text(String::from("Help: ") + hint)
                .size(SMALL_TEXT)
                .horizontal_alignment(Horizontal::Center),
        )
        .push(vertical_space(Length::Fill))
        .push(
            row()
                .spacing(SPACING)
                .push(
                    make_button("CANCEL")
                        .style(style::Button::Red)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: true }),
                )
                .push(horizontal_space(Length::Fill))
                .push(
                    make_button("DONE")
                        .style(style::Button::Green)
                        .width(Length::Fill)
                        .on_press(Message::ParameterEditComplete { canceled: false }),
                ),
        )
        .into()
}
