use super::{
    super::{
        super::{snapshot::BeepTestSnapshot, sound_controller::*},
        message::*,
    },
    shared_elements::*,
    *,
};
use iced::{
    Element, Length,
    widget::{column, horizontal_space, row},
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in super::super) struct EditableSettings {
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

pub fn make_sound_config_page<'a>(
    snapshot: &BeepTestSnapshot,
    edited_settings: &EditableSettings,
) -> Element<'a, Message> {
    let EditableSettings { sound, .. } = edited_settings;

    column![
        make_time_button(snapshot),
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
        ]
        .spacing(SPACING)
        .height(Length::Fill),
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
        ]
        .spacing(SPACING)
        .height(Length::Fill),
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
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            horizontal_space(),
            horizontal_space(),
            make_button("DONE")
                .style(green_button)
                .width(Length::Fill)
                .on_press(Message::EditComplete),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}
