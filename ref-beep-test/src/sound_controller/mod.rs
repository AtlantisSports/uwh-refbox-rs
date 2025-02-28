use crate::app::update_sender::ServerMessage;
use derivative::Derivative;
use enum_derive_2018::{EnumDisplay, EnumFromStr};
use log::*;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{
    sync::{
        mpsc::{UnboundedSender, unbounded_channel},
        watch::{self, Sender},
    },
    task::{self, JoinHandle},
    time::{Duration, sleep},
};
use toml::Table;
use web_audio_api::{
    AudioBuffer,
    context::{AudioContext, AudioContextOptions, BaseAudioContext},
    node::{
        AudioBufferSourceNode, AudioNode, AudioScheduledSourceNode, ChannelInterpretation,
        ChannelMergerNode, GainNode,
    },
};

const FADE_LEN: f64 = 0.05;
const FADE_WAIT: Duration = Duration::from_millis(50); // TODO: base this on `FADE_TIME` (blocked on rust allowing floats in const fns)

const SOUND_LEN: f64 = 2.0;

mod sounds;
pub use sounds::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SoundSettings {
    #[derivative(Default(value = "true"))]
    pub sound_enabled: bool,
    #[derivative(Default(value = "true"))]
    pub whistle_enabled: bool,
    pub buzzer_sound: BuzzerSound,
    #[derivative(Default(value = "Volume::Medium"))]
    pub whistle_vol: Volume,
    pub above_water_vol: Volume,
    pub under_water_vol: Volume,
    #[derivative(Default(value = "true"))]
    pub auto_sound_start_play: bool,
    #[derivative(Default(value = "true"))]
    pub auto_sound_stop_play: bool,
    pub remotes: Vec<RemoteInfo>,
}

impl SoundSettings {
    pub fn migrate(old: &Table) -> Self {
        let Self {
            mut sound_enabled,
            mut whistle_enabled,
            mut buzzer_sound,
            mut whistle_vol,
            mut above_water_vol,
            mut under_water_vol,
            mut auto_sound_start_play,
            mut auto_sound_stop_play,
            mut remotes,
        } = Default::default();

        if let Some(old_sound_enabled) = old.get("sound_enabled") {
            if let Some(old_sound_enabled) = old_sound_enabled.as_bool() {
                sound_enabled = old_sound_enabled;
            }
        }
        if let Some(old_whistle_enabled) = old.get("whistle_enabled") {
            if let Some(old_whistle_enabled) = old_whistle_enabled.as_bool() {
                whistle_enabled = old_whistle_enabled;
            }
        }
        if let Some(old_buzzer_sound) = old.get("buzzer_sound") {
            if let Some(old_buzzer_sound) = old_buzzer_sound.as_str() {
                if let Ok(sound) = old_buzzer_sound.parse() {
                    buzzer_sound = sound;
                }
            }
        }
        if let Some(old_whistle_vol) = old.get("whistle_vol") {
            if let Some(old_whistle_vol) = old_whistle_vol.as_str() {
                if let Ok(vol) = old_whistle_vol.parse() {
                    whistle_vol = vol;
                }
            }
        }
        if let Some(old_above_water_vol) = old.get("above_water_vol") {
            if let Some(old_above_water_vol) = old_above_water_vol.as_str() {
                if let Ok(vol) = old_above_water_vol.parse() {
                    above_water_vol = vol;
                }
            }
        }
        if let Some(old_under_water_vol) = old.get("under_water_vol") {
            if let Some(old_under_water_vol) = old_under_water_vol.as_str() {
                if let Ok(vol) = old_under_water_vol.parse() {
                    under_water_vol = vol;
                }
            }
        }
        if let Some(old_auto_sound_start_play) = old.get("auto_sound_start_play") {
            if let Some(old_auto_sound_start_play) = old_auto_sound_start_play.as_bool() {
                auto_sound_start_play = old_auto_sound_start_play;
            }
        }
        if let Some(old_auto_sound_stop_play) = old.get("auto_sound_stop_play") {
            if let Some(old_auto_sound_stop_play) = old_auto_sound_stop_play.as_bool() {
                auto_sound_stop_play = old_auto_sound_stop_play;
            }
        }
        if let Some(old_remotes) = old.get("remotes") {
            if let Some(old_remotes) = old_remotes.as_array() {
                remotes = old_remotes
                    .iter()
                    .filter_map(|r| {
                        if let Some(r) = r.as_table() {
                            let id = r.get("id")?.as_integer()? as u32;
                            let sound = r.get("sound")?.as_str()?.parse().ok();
                            Some(RemoteInfo { id, sound })
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }

        Self {
            sound_enabled,
            whistle_enabled,
            buzzer_sound,
            whistle_vol,
            above_water_vol,
            under_water_vol,
            auto_sound_start_play,
            auto_sound_stop_play,
            remotes,
        }
    }
}

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!, EnumFromStr!)]
    #[derivative(Default)]
    pub enum Volume {
        Off,
        Low,
        Medium,
        High,
        #[derivative(Default)]
        Max,
    }
}

impl Volume {
    fn as_f32(&self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Low => 10f32.powf(-1.2),    // 12dB lower than max
            Self::Medium => 10f32.powf(-0.8), // 8dB lower than max
            Self::High => 10f32.powf(-0.4),   // 4dB lower than max
            Self::Max => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub id: u32,
    pub sound: Option<BuzzerSound>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SoundMessage {
    TriggerBuzzer,
    TriggerWhistle,
}

pub struct SoundController {
    _context: Arc<AudioContext>,
    msg_tx: UnboundedSender<SoundMessage>,
    settings_tx: Sender<SoundSettings>,
    stop_tx: Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

impl SoundController {
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    pub fn new<F>(settings: SoundSettings, trigger_flash: F) -> Self
    where
        F: Send
            + Fn() -> Result<(), tokio::sync::mpsc::error::TrySendError<ServerMessage>>
            + 'static,
    {
        let opts = AudioContextOptions {
            sample_rate: Some(SAMPLE_RATE),
            ..AudioContextOptions::default()
        };

        let context = Arc::new(AudioContext::new(opts));

        let library = SoundLibrary::new(&context);

        let (msg_tx, mut msg_rx) = unbounded_channel();

        let (settings_tx, mut settings_rx) = watch::channel(settings.clone());
        settings_rx.borrow_and_update();

        let (stop_tx, mut stop_rx) = watch::channel(false);
        stop_rx.borrow_and_update();

        let mut _stop_rx = stop_rx.clone();
        let mut _settings_rx = settings_rx.clone();
        #[cfg_attr(not(target_os = "linux"), allow(clippy::redundant_clone))]
        let mut _settings = settings.clone();
        let _context = context.clone();

        let handler = task::spawn(async move {
            #[cfg_attr(not(target_os = "linux"), allow(unused_assignments))]
            let mut last_sound: Option<Sound> = None;

            loop {
                tokio::select! {
                    msg = msg_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if let Some(sound) = last_sound.take() {
                                    sound.stop().await;
                                }

                                match msg {
                                    SoundMessage::TriggerBuzzer => {
                                        info!("Auto-triggering buzzer");
                                        let volumes = ChannelVolumes::new(&_settings, false);
                                        let sound = Sound::new(_context.clone(), volumes, library[_settings.buzzer_sound].clone(), true, true);
                                        trigger_flash().unwrap();
                                        last_sound = Some(sound);
                                    }
                                    SoundMessage::TriggerWhistle => {
                                        info!("Playing whistle once");
                                        let volumes = ChannelVolumes::new(&_settings, true);
                                        let sound = Sound::new(_context.clone(), volumes, library.whistle().clone(), false, false);
                                        last_sound = Some(sound);
                                    }

                                }
                            },
                            None => break,
                        }
                    }
                    maybe_err = _settings_rx.changed() => {
                        match maybe_err {
                            Ok(()) => {
                                _settings = _settings_rx.borrow().clone();
                            }
                            Err(_) => break,
                        }
                    }
                    _ = _stop_rx.changed() => {
                        break;
                    }
                }
            }
        });

        #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
        let tasks = vec![handler];

        Self {
            _context: context,
            msg_tx,
            settings_tx,
            stop_tx,
            tasks,
        }
    }

    pub fn update_settings(&self, settings: SoundSettings) {
        self.settings_tx.send(settings).unwrap()
    }

    pub fn trigger_whistle(&self) {
        self.msg_tx.send(SoundMessage::TriggerWhistle).unwrap()
    }

    pub fn trigger_buzzer(&self) {
        self.msg_tx.send(SoundMessage::TriggerBuzzer).unwrap()
    }
}

impl Drop for SoundController {
    fn drop(&mut self) {
        if self.stop_tx.send(true).is_err() {
            return;
        }

        tokio::runtime::Handle::current().block_on(async {
            for join_handle in self.tasks.drain(..) {
                join_handle.await.unwrap();
            }
        });
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct ChannelVolumes {
    left: f32,
    right: f32,
}

impl ChannelVolumes {
    fn new(settings: &SoundSettings, is_whistle: bool) -> Self {
        Self {
            left: if settings.sound_enabled && settings.whistle_enabled && is_whistle {
                settings.whistle_vol.as_f32()
            } else if settings.sound_enabled && !is_whistle {
                settings.above_water_vol.as_f32()
            } else {
                0.0
            },
            right: if settings.sound_enabled && !is_whistle {
                settings.under_water_vol.as_f32()
            } else {
                0.0
            },
        }
    }
}

struct Sound {
    _merger: ChannelMergerNode,
    gain_l: GainNode,
    gain_r: GainNode,
    source: AudioBufferSourceNode,
    context: Arc<AudioContext>,
    volumes: ChannelVolumes,
}

impl Sound {
    fn new(
        context: Arc<AudioContext>,
        volumes: ChannelVolumes,
        buffer: AudioBuffer,
        repeat: bool,
        timed: bool,
    ) -> Self {
        let _merger = context.create_channel_merger(2);
        _merger.set_channel_interpretation(ChannelInterpretation::Speakers);
        _merger.connect(&context.destination());

        let gain_l = context.create_gain();
        gain_l.connect_from_output_to_input(&_merger, 0, 0);
        gain_l.gain().set_value(volumes.left);

        let gain_r = context.create_gain();
        gain_r.connect_from_output_to_input(&_merger, 0, 1);
        gain_r.gain().set_value(volumes.right);

        let mut source = context.create_buffer_source();
        source.set_buffer(buffer);
        source.connect(&gain_l);
        source.connect(&gain_r);
        source.set_loop(repeat);

        let fade_end = context.current_time() + FADE_LEN;

        // Set the gains so that the start of the fade is now
        gain_l.gain().set_value(0.0);
        gain_r.gain().set_value(0.0);

        gain_l
            .gain()
            .linear_ramp_to_value_at_time(volumes.left, fade_end);
        gain_r
            .gain()
            .linear_ramp_to_value_at_time(volumes.right, fade_end);

        if timed {
            let sound_end = fade_end + SOUND_LEN;
            let fade_out_end = sound_end + FADE_LEN;

            gain_l.gain().set_value_at_time(volumes.left, sound_end);
            gain_l
                .gain()
                .linear_ramp_to_value_at_time(0.0, fade_out_end);

            gain_r.gain().set_value_at_time(volumes.right, sound_end);
            gain_r
                .gain()
                .linear_ramp_to_value_at_time(0.0, fade_out_end);
        }

        source.start();

        Self {
            _merger,
            gain_l,
            gain_r,
            source,
            context,
            volumes,
        }
    }

    async fn stop(mut self) {
        let fade_end = self.context.current_time() + FADE_LEN;

        // Set the gains so that the start of the fade is now, not when the sound started
        self.gain_l.gain().set_value(self.volumes.left);
        self.gain_r.gain().set_value(self.volumes.right);

        self.gain_l
            .gain()
            .linear_ramp_to_value_at_time(0.0, fade_end);
        self.gain_r
            .gain()
            .linear_ramp_to_value_at_time(0.0, fade_end);

        sleep(FADE_WAIT).await;
        self.source.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser_sound_settings() {
        let settings: SoundSettings = Default::default();
        let serialized = toml::to_string(&settings).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(settings));
    }

    #[test]
    fn test_ser_volume() {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Test {
            vol: Volume,
        }

        let vol = Test { vol: Volume::Off };
        let serialized = toml::to_string(&vol).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(vol));

        let vol = Test { vol: Volume::Low };
        let serialized = toml::to_string(&vol).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(vol));

        let vol = Test {
            vol: Volume::Medium,
        };
        let serialized = toml::to_string(&vol).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(vol));

        let vol = Test { vol: Volume::High };
        let serialized = toml::to_string(&vol).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(vol));

        let vol = Test { vol: Volume::Max };
        let serialized = toml::to_string(&vol).unwrap();
        let deser = toml::from_str(&serialized);
        assert_eq!(deser, Ok(vol));
    }

    #[test]
    fn test_migrate_sound_settings() {
        let mut old = Table::new();
        old.insert("sound_enabled".to_string(), toml::Value::Boolean(false));
        old.insert("whistle_enabled".to_string(), toml::Value::Boolean(false));
        old.insert(
            "buzzer_sound".to_string(),
            toml::Value::String("Buzz".to_string()),
        );
        old.insert(
            "whistle_vol".to_string(),
            toml::Value::String("Low".to_string()),
        );
        old.insert(
            "above_water_vol".to_string(),
            toml::Value::String("Medium".to_string()),
        );
        old.insert(
            "under_water_vol".to_string(),
            toml::Value::String("Medium".to_string()),
        );
        old.insert(
            "auto_sound_start_play".to_string(),
            toml::Value::Boolean(false),
        );
        old.insert(
            "auto_sound_stop_play".to_string(),
            toml::Value::Boolean(false),
        );
        old.insert(
            "remotes".to_string(),
            toml::Value::Array(vec![
                toml::Value::Table(
                    vec![
                        ("id".to_string(), toml::Value::Integer(1)),
                        ("sound".to_string(), toml::Value::String("Buzz".to_string())),
                    ]
                    .into_iter()
                    .collect(),
                ),
                toml::Value::Table(
                    vec![
                        ("id".to_string(), toml::Value::Integer(2)),
                        (
                            "sound".to_string(),
                            toml::Value::String("DeDeDu".to_string()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            ]),
        );

        let settings = SoundSettings::migrate(&old);

        assert_eq!(settings.sound_enabled, false);
        assert_eq!(settings.whistle_enabled, false);
        assert_eq!(settings.buzzer_sound, BuzzerSound::Buzz);
        assert_eq!(settings.whistle_vol, Volume::Low);
        assert_eq!(settings.above_water_vol, Volume::Medium);
        assert_eq!(settings.under_water_vol, Volume::Medium);
        assert_eq!(settings.auto_sound_start_play, false);
        assert_eq!(settings.auto_sound_stop_play, false);
        assert_eq!(
            settings.remotes,
            vec![
                RemoteInfo {
                    id: 1,
                    sound: Some(BuzzerSound::Buzz),
                },
                RemoteInfo {
                    id: 2,
                    sound: Some(BuzzerSound::DeDeDu),
                },
            ]
        );
    }
}
