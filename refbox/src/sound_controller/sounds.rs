use derivative::Derivative;
use enum_derive_2018::EnumFromStr;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Index};
use web_audio_api::{
    context::{AudioContext, BaseAudioContext},
    AudioBuffer,
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

pub const SAMPLE_RATE: f32 = 44100.0;

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumFromStr!)]
    #[derivative(Default)]
    pub enum BuzzerSound {
        #[derivative(Default)]
        Buzz,
        Whoop,
        Crazy,
        DeDeDu,
        TwoTone,
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
        }
    }
}

pub(super) struct SoundLibrary {
    buzz: AudioBuffer,
    whoop: AudioBuffer,
    crazy: AudioBuffer,
    de_de_du: AudioBuffer,
    two_tone: AudioBuffer,
    whistle: AudioBuffer,
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

        Self {
            buzz,
            whoop,
            crazy,
            de_de_du,
            two_tone,
            whistle,
        }
    }

    pub(super) fn whistle(&self) -> &AudioBuffer {
        &self.whistle
    }
}
