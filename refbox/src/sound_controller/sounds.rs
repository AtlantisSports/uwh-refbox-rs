use array_concat::*;
use derivative::Derivative;
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
        output[i] = unsafe {
            std::mem::transmute(u32::from_le_bytes([
                input[i * 4],
                input[(i * 4) + 1],
                input[(i * 4) + 2],
                input[(i * 4) + 3],
            ]))
        };
        i += 1;
    }

    output
}

// Until there is a resolution to https://github.com/rust-lang/rust/issues/93481, `process_array()`
// can't be run on sound samples more than about 0.8s long, so we need to work around it by
// processing smaller samples then concatenating them. See https://github.com/rust-lang/rust/pull/103877
// for a resolution that will potentially land soon
const LEN1: usize = include_bytes!("../../resources/sounds/ref-alert-1.raw").len() / 4;
const PT1: [f32; LEN1] = process_array(include_bytes!("../../resources/sounds/ref-alert-1.raw"));
const LEN2: usize = include_bytes!("../../resources/sounds/ref-alert-2.raw").len() / 4;
const PT2: [f32; LEN2] = process_array(include_bytes!("../../resources/sounds/ref-alert-2.raw"));
const CONCAT: [f32; concat_arrays_size!(PT1, PT2)] = concat_arrays!(PT1, PT2);
const REF_ALERT_LEN: usize = concat_arrays_size!(CONCAT, PT1);
static REF_ALERT: [f32; REF_ALERT_LEN] = concat_arrays!(CONCAT, PT1);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum BuzzerSound {
    #[derivative(Default)]
    Buzz,
    Whoop,
    Crazy,
    DeDeDu,
    TwoTone,
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
    ref_alert: AudioBuffer,
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

        let mut ref_alert = context.create_buffer(1, REF_ALERT_LEN, SAMPLE_RATE);
        ref_alert.copy_to_channel(&REF_ALERT, 0);

        Self {
            buzz,
            whoop,
            crazy,
            de_de_du,
            two_tone,
            ref_alert,
        }
    }

    pub(super) fn ref_alert(&self) -> &AudioBuffer {
        &self.ref_alert
    }
}
