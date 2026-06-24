use derivative::Derivative;
use enum_derive_2018::EnumFromStr;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Index};
use web_audio_api::{
    AudioBuffer,
    context::{AudioContext, BaseAudioContext},
};

const fn process_array<const N: usize, const M: usize>(input: &[u8; M]) -> [f32; N] {
    let mut output = [0f32; N];

    let mut i = 0;
    while i < N {
        output[i] = f32::from_bits(u32::from_le_bytes([
            input[i * 4],
            input[(i * 4) + 1],
            input[(i * 4) + 2],
            input[(i * 4) + 3],
        ]));
        i += 1;
    }

    output
}

const WHISTLE_LEN: usize = include_bytes!("../../resources/sounds/whistle.raw").len() / 4;
static WHISTLE: [f32; WHISTLE_LEN] =
    process_array(include_bytes!("../../resources/sounds/whistle.raw"));

const BUZZ_LEN: usize = include_bytes!("../../resources/sounds/buzz.raw").len() / 4;
static BUZZ: [f32; BUZZ_LEN] = process_array(include_bytes!("../../resources/sounds/buzz.raw"));

const WHOOP_LEN: usize = include_bytes!("../../resources/sounds/whoop.raw").len() / 4;
static WHOOP: [f32; WHOOP_LEN] = process_array(include_bytes!("../../resources/sounds/whoop.raw"));

const CRAZY_LEN: usize = include_bytes!("../../resources/sounds/crazy.raw").len() / 4;
static CRAZY: [f32; CRAZY_LEN] = process_array(include_bytes!("../../resources/sounds/crazy.raw"));

const DE_DE_DU_LEN: usize = include_bytes!("../../resources/sounds/de-de-du.raw").len() / 4;
static DE_DE_DU: [f32; DE_DE_DU_LEN] =
    process_array(include_bytes!("../../resources/sounds/de-de-du.raw"));

const TWO_TONE_LEN: usize = include_bytes!("../../resources/sounds/two-tone.raw").len() / 4;
static TWO_TONE: [f32; TWO_TONE_LEN] =
    process_array(include_bytes!("../../resources/sounds/two-tone.raw"));

const COUNTDOWN_LEN: usize = include_bytes!("../../resources/sounds/countdown.raw").len() / 4;
static COUNTDOWN: [f32; COUNTDOWN_LEN] =
    process_array(include_bytes!("../../resources/sounds/countdown.raw"));

const AIRHORN_LEN: usize = include_bytes!("../../resources/sounds/airhorn.raw").len() / 4;
static AIRHORN: [f32; AIRHORN_LEN] =
    process_array(include_bytes!("../../resources/sounds/airhorn.raw"));

const PIPES_LEN: usize = include_bytes!("../../resources/sounds/pipes.raw").len() / 4;
static PIPES: [f32; PIPES_LEN] = process_array(include_bytes!("../../resources/sounds/pipes.raw"));

const KLAXON_LEN: usize = include_bytes!("../../resources/sounds/klaxon.raw").len() / 4;
static KLAXON: [f32; KLAXON_LEN] =
    process_array(include_bytes!("../../resources/sounds/klaxon.raw"));

const PIP_LEN: usize = include_bytes!("../../resources/sounds/pip.raw").len() / 4;
static PIP: [f32; PIP_LEN] = process_array(include_bytes!("../../resources/sounds/pip.raw"));

const PULSE_LEN: usize = include_bytes!("../../resources/sounds/pulse.raw").len() / 4;
static PULSE: [f32; PULSE_LEN] = process_array(include_bytes!("../../resources/sounds/pulse.raw"));

const SIREN_LEN: usize = include_bytes!("../../resources/sounds/siren.raw").len() / 4;
static SIREN: [f32; SIREN_LEN] = process_array(include_bytes!("../../resources/sounds/siren.raw"));

const TRILL_LEN: usize = include_bytes!("../../resources/sounds/trill.raw").len() / 4;
static TRILL: [f32; TRILL_LEN] = process_array(include_bytes!("../../resources/sounds/trill.raw"));

pub const SAMPLE_RATE: f32 = 44100.0;

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Derivative, EnumFromStr!)]
    #[derivative(Default)]
    pub enum BuzzerSound {
        #[derivative(Default)]
        Buzz,
        Whoop,
        Crazy,
        DeDeDu,
        TwoTone,
        Airhorn,
        Pipes,
        Klaxon,
        Pip,
        Pulse,
        Siren,
        Trill,
    }
}

impl Display for BuzzerSound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buzz => write!(f, "Buzz"),
            Self::Whoop => write!(f, "Whoop"),
            Self::Crazy => write!(f, "Crazy"),
            Self::DeDeDu => write!(f, "De De Du"),
            Self::TwoTone => write!(f, "Two Tone"),
            Self::Airhorn => write!(f, "Airhorn"),
            Self::Pipes => write!(f, "Pipes"),
            Self::Klaxon => write!(f, "Klaxon"),
            Self::Pip => write!(f, "Pip"),
            Self::Pulse => write!(f, "Pulse"),
            Self::Siren => write!(f, "Siren"),
            Self::Trill => write!(f, "Trill"),
        }
    }
}

impl BuzzerSound {
    /// All buzzer sounds, in picker display order (existing first, new last).
    // Used by the sounds picker UI (wired in a later task).
    #[allow(dead_code)]
    pub const ALL: [BuzzerSound; 12] = [
        BuzzerSound::Buzz,
        BuzzerSound::Whoop,
        BuzzerSound::Crazy,
        BuzzerSound::DeDeDu,
        BuzzerSound::TwoTone,
        BuzzerSound::Airhorn,
        BuzzerSound::Pipes,
        BuzzerSound::Klaxon,
        BuzzerSound::Pip,
        BuzzerSound::Pulse,
        BuzzerSound::Siren,
        BuzzerSound::Trill,
    ];
}

pub(super) struct SoundLibrary {
    buzz: AudioBuffer,
    whoop: AudioBuffer,
    crazy: AudioBuffer,
    de_de_du: AudioBuffer,
    two_tone: AudioBuffer,
    whistle: AudioBuffer,
    countdown: AudioBuffer,
    airhorn: AudioBuffer,
    pipes: AudioBuffer,
    klaxon: AudioBuffer,
    pip: AudioBuffer,
    pulse: AudioBuffer,
    siren: AudioBuffer,
    trill: AudioBuffer,
}

impl Index<BuzzerSound> for SoundLibrary {
    type Output = AudioBuffer;

    fn index(&self, sound: BuzzerSound) -> &Self::Output {
        match sound {
            BuzzerSound::Buzz => &self.buzz,
            BuzzerSound::Whoop => &self.whoop,
            BuzzerSound::Crazy => &self.crazy,
            BuzzerSound::DeDeDu => &self.de_de_du,
            BuzzerSound::TwoTone => &self.two_tone,
            BuzzerSound::Airhorn => &self.airhorn,
            BuzzerSound::Pipes => &self.pipes,
            BuzzerSound::Klaxon => &self.klaxon,
            BuzzerSound::Pip => &self.pip,
            BuzzerSound::Pulse => &self.pulse,
            BuzzerSound::Siren => &self.siren,
            BuzzerSound::Trill => &self.trill,
        }
    }
}

impl SoundLibrary {
    pub(super) fn new(context: &AudioContext) -> Self {
        let mut buzz = context.create_buffer(1, BUZZ_LEN, SAMPLE_RATE);
        buzz.copy_to_channel(&BUZZ, 0);

        let mut whoop = context.create_buffer(1, WHOOP_LEN, SAMPLE_RATE);
        whoop.copy_to_channel(&WHOOP, 0);

        let mut crazy = context.create_buffer(1, CRAZY_LEN, SAMPLE_RATE);
        crazy.copy_to_channel(&CRAZY, 0);

        let mut de_de_du = context.create_buffer(1, DE_DE_DU_LEN, SAMPLE_RATE);
        de_de_du.copy_to_channel(&DE_DE_DU, 0);

        let mut two_tone = context.create_buffer(1, TWO_TONE_LEN, SAMPLE_RATE);
        two_tone.copy_to_channel(&TWO_TONE, 0);

        let mut whistle = context.create_buffer(1, WHISTLE_LEN, SAMPLE_RATE);
        whistle.copy_to_channel(&WHISTLE, 0);

        let mut countdown = context.create_buffer(1, COUNTDOWN_LEN, SAMPLE_RATE);
        countdown.copy_to_channel(&COUNTDOWN, 0);

        let mut airhorn = context.create_buffer(1, AIRHORN_LEN, SAMPLE_RATE);
        airhorn.copy_to_channel(&AIRHORN, 0);

        let mut pipes = context.create_buffer(1, PIPES_LEN, SAMPLE_RATE);
        pipes.copy_to_channel(&PIPES, 0);

        let mut klaxon = context.create_buffer(1, KLAXON_LEN, SAMPLE_RATE);
        klaxon.copy_to_channel(&KLAXON, 0);

        let mut pip = context.create_buffer(1, PIP_LEN, SAMPLE_RATE);
        pip.copy_to_channel(&PIP, 0);

        let mut pulse = context.create_buffer(1, PULSE_LEN, SAMPLE_RATE);
        pulse.copy_to_channel(&PULSE, 0);

        let mut siren = context.create_buffer(1, SIREN_LEN, SAMPLE_RATE);
        siren.copy_to_channel(&SIREN, 0);

        let mut trill = context.create_buffer(1, TRILL_LEN, SAMPLE_RATE);
        trill.copy_to_channel(&TRILL, 0);

        Self {
            buzz,
            whoop,
            crazy,
            de_de_du,
            two_tone,
            whistle,
            countdown,
            airhorn,
            pipes,
            klaxon,
            pip,
            pulse,
            siren,
            trill,
        }
    }

    pub(super) fn whistle(&self) -> &AudioBuffer {
        &self.whistle
    }

    pub(super) fn countdown(&self) -> &AudioBuffer {
        &self.countdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_buzzer_sounds_round_trip_via_serde() {
        for s in BuzzerSound::ALL {
            let toml = toml::to_string(&Wrap { s }).unwrap();
            let back: Wrap = toml::from_str(&toml).unwrap();
            assert_eq!(back.s, s, "round-trip failed for {s:?}");
        }
        assert_eq!(BuzzerSound::ALL.len(), 12);
    }
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
    struct Wrap {
        s: BuzzerSound,
    }
}
