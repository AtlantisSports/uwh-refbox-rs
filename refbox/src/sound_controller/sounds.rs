use array_concat::*;
use derivative::Derivative;
use enum_derive_2018::EnumDisplay;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};
use std::ops::Index;
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
const LEN1: usize = include_bytes!("../../resources/sounds/ref-warn-1.raw").len() / 4;
const PT1: [f32; LEN1] = process_array(include_bytes!("../../resources/sounds/ref-warn-1.raw"));
const LEN2: usize = include_bytes!("../../resources/sounds/ref-warn-2.raw").len() / 4;
const PT2: [f32; LEN2] = process_array(include_bytes!("../../resources/sounds/ref-warn-2.raw"));
const CONCAT: [f32; concat_arrays_size!(PT1, PT2)] = concat_arrays!(PT1, PT2);
static REF_WARN: [f32; concat_arrays_size!(CONCAT, PT1)] = concat_arrays!(CONCAT, PT1);

const BEEP_LEN: usize = include_bytes!("../../resources/sounds/beep.raw").len() / 4;
static BEEP: [f32; BEEP_LEN] = process_array(include_bytes!("../../resources/sounds/beep.raw"));

const WHOOP_LEN: usize = include_bytes!("../../resources/sounds/whoop.raw").len() / 4;
static WHOOP: [f32; WHOOP_LEN] = process_array(include_bytes!("../../resources/sounds/whoop.raw"));

pub const SAMPLE_RATE: f32 = 44100.0;

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!)]
    #[derivative(Default)]
    pub enum BuzzerSound {
        #[derivative(Default)]
        Beep,
        Whoop,
    }
}

pub(super) struct SoundLibrary {
    beep: AudioBuffer,
    whoop: AudioBuffer,
    ref_warn: AudioBuffer,
}

impl Index<BuzzerSound> for SoundLibrary {
    type Output = AudioBuffer;

    fn index(&self, sound: BuzzerSound) -> &Self::Output {
        match sound {
            BuzzerSound::Beep => &self.beep,
            BuzzerSound::Whoop => &self.whoop,
        }
    }
}

impl SoundLibrary {
    pub(super) fn new(context: &AudioContext) -> Self {
        let mut beep = context.create_buffer(1, BEEP.len(), SAMPLE_RATE);
        beep.copy_to_channel(&BEEP, 0);

        let mut whoop = context.create_buffer(1, WHOOP.len(), SAMPLE_RATE);
        whoop.copy_to_channel(&WHOOP, 0);

        let mut ref_warn = context.create_buffer(1, REF_WARN.len(), SAMPLE_RATE);
        ref_warn.copy_to_channel(&REF_WARN, 0);

        Self {
            beep,
            whoop,
            ref_warn,
        }
    }

    pub(super) fn ref_warn(&self) -> &AudioBuffer {
        &self.ref_warn
    }
}
