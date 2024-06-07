use super::fl;
#[cfg(target_os = "linux")]
use arrayref::array_ref;
#[cfg(target_os = "linux")]
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use derivative::Derivative;
use enum_derive_2018::EnumFromStr;
#[cfg(target_os = "linux")]
use futures_lite::future::FutureExt;
use log::*;
use macro_attr_2018::macro_attr;
#[cfg(target_os = "linux")]
use rppal::gpio::{Gpio, InputPin, Level, Trigger};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};
#[cfg(target_os = "linux")]
use tokio::{
    sync::watch::Receiver,
    time::{sleep_until, Instant},
};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        watch::{self, Sender},
    },
    task::{self, JoinHandle},
    time::{sleep, Duration},
};
use toml::Table;
use web_audio_api::{
    context::{AudioContext, AudioContextOptions, BaseAudioContext},
    node::{
        AudioBufferSourceNode, AudioNode, AudioScheduledSourceNode, ChannelInterpretation,
        ChannelMergerNode, GainNode,
    },
    AudioBuffer,
};

const FADE_LEN: f64 = 0.05;
const FADE_WAIT: Duration = Duration::from_millis(50); // TODO: base this on `FADE_TIME` (blocked on rust allowing floats in const fns)

const SOUND_LEN: f64 = 2.0;

#[cfg(target_os = "linux")]
const MESSAGE_LEN: usize = 24;
#[cfg(target_os = "linux")]
const ID_LEN: usize = 20;
#[cfg(target_os = "linux")]
const DATA_LEN: usize = MESSAGE_LEN - ID_LEN;

#[cfg(target_os = "linux")]
const BUTTON_TIMEOUT: Duration = Duration::from_millis(500);

mod sounds;
pub use sounds::*;

use crate::app::update_sender::ServerMessage;

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumFromStr!)]
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

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => f.write_str(&fl!("off")),
            Self::Low => f.write_str(&fl!("low")),
            Self::Medium => f.write_str(&fl!("medium")),
            Self::High => f.write_str(&fl!("high")),
            Self::Max => f.write_str(&fl!("max")),
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
    #[cfg(target_os = "linux")]
    StartBuzzer(Option<BuzzerSound>),
    #[cfg(target_os = "linux")]
    StopBuzzer,
}

pub struct SoundController {
    _context: Arc<AudioContext>,
    msg_tx: UnboundedSender<SoundMessage>,
    settings_tx: Sender<SoundSettings>,
    stop_tx: Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    remote_id_rx: Option<Receiver<u32>>,
    #[cfg(target_os = "linux")]
    _pins: Option<(InputPin, InputPin)>,
}

impl SoundController {
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    pub fn new<F>(mut settings: SoundSettings, trigger_flash: F) -> Self
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
                                    #[cfg(target_os = "linux")]
                                    SoundMessage::StartBuzzer(sound_option) => {
                                        info!("Starting buzzer");
                                        let buzzer_sound = sound_option.unwrap_or(_settings.buzzer_sound);
                                        let volumes = ChannelVolumes::new(&_settings, false);
                                        let sound = Sound::new(_context.clone(), volumes, library[buzzer_sound].clone(), true, false);
                                        trigger_flash().unwrap();
                                        last_sound = Some(sound);
                                    }
                                    #[cfg(target_os = "linux")]
                                    SoundMessage::StopBuzzer => {
                                        info!("Stopped buzzer");
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
        let mut tasks = vec![handler];

        #[cfg(target_os = "linux")]
        let (_pins, remote_id_rx) = if let Ok(sys_info) = rppal::system::DeviceInfo::new() {
            info!("Detected a Raspberry Pi system: {sys_info:?}, starting GPIO processes");

            let gpio = Gpio::new().unwrap();

            let mut ant_pin = gpio.get(16).unwrap().into_input_pullup();
            let (ant_tx, mut ant_rx) = unbounded_channel();
            ant_pin
                .set_async_interrupt(Trigger::Both, move |level| {
                    ant_tx.send((level, Instant::now())).unwrap()
                })
                .unwrap();

            let mut wired_pin = gpio.get(12).unwrap().into_input_pullup();
            let (wired_tx, mut wired_rx) = unbounded_channel();
            wired_pin
                .set_async_interrupt(Trigger::Both, move |level| wired_tx.send(level).unwrap())
                .unwrap();

            let ant_start_state = ant_pin.read();

            let (wireless_tx, mut wireless_rx) = unbounded_channel();
            let (remote_id_tx, mut remote_id_rx) = watch::channel(0);
            remote_id_rx.borrow_and_update();

            let mut _stop_rx = stop_rx.clone();

            let wireless_button_listener = task::spawn(async move {
                let mut state = RemoteDetectorState {
                    preamble_detected: false,
                    bits: vec![],
                    last_pin_state: ant_start_state,
                    last_edge_time: Instant::now(),
                    last_pulse: None,
                };

                loop {
                    tokio::select! {
                        pin_update = ant_rx.recv() => {
                            match pin_update {
                                Some((l @ Level::High, now)) => {
                                    if state.last_pin_state != l {
                                        let pulse = now.duration_since(state.last_edge_time).as_micros();
                                        trace!("Detected LOW  pulse {pulse:>5}us long");
                                        state.last_pin_state = l;
                                        state.last_edge_time = now;

                                        let maybe_pulse_type = identify_pulse(pulse);
                                        debug!("Detected a LOW  pulse of length {maybe_pulse_type:?}");

                                        if let Some(pulse_type) = maybe_pulse_type {
                                            if !state.preamble_detected {
                                                if matches!(state.last_pulse, Some(PulseType::Short))
                                                    && pulse_type == PulseType::Preamble
                                                {
                                                    debug!("Detected a preamble");
                                                    state.preamble_detected = true;
                                                }
                                            } else {
                                                if matches!(state.last_pulse, Some(PulseType::Short))
                                                    && pulse_type == PulseType::Long
                                                {
                                                    debug!("Detected a low bit");
                                                    state.bits.push(false);
                                                } else if matches!(state.last_pulse, Some(PulseType::Long))
                                                    && pulse_type == PulseType::Short
                                                {
                                                    debug!("Detected a high bit");
                                                    state.bits.push(true);
                                                } else {
                                                    info!("Detected an invalid pulse sequence");
                                                    state.preamble_detected = false;
                                                    state.bits.clear();
                                                }

                                                if state.bits.len() == MESSAGE_LEN {
                                                    let message: String = state
                                                        .bits
                                                        .iter()
                                                        .map(|bit| if *bit { '1' } else { '0' })
                                                        .collect();
                                                    debug!("Received a complete message: 0b{message}");

                                                    let remote_id = state.bits[..ID_LEN]
                                                        .iter()
                                                        .fold(0, |acc, &b| acc * 2 + b as u32);
                                                    let data = array_ref![state.bits, ID_LEN, DATA_LEN];

                                                    debug!("Remote {remote_id} sent data {data:?}");
                                                    wireless_tx.send(remote_id).unwrap();
                                                    remote_id_tx.send(remote_id).unwrap();

                                                    state.preamble_detected = false;
                                                    state.bits.clear();
                                                }
                                            }
                                        } else {
                                            trace!("Detected an invalid pulse");
                                            state.preamble_detected = false;
                                            state.bits.clear();
                                        }

                                        state.last_pulse = maybe_pulse_type;
                                    }
                                }
                                Some((l @ Level::Low, now)) => {
                                    if state.last_pin_state != l {
                                        let pulse = now.duration_since(state.last_edge_time).as_micros();
                                        trace!("Detected HIGH pulse {pulse:>5}us long");
                                        state.last_pin_state = l;
                                        state.last_edge_time = now;

                                        let maybe_pulse_type = identify_pulse(pulse);
                                        debug!("Detected a LOW  pulse of length {maybe_pulse_type:?}");

                                        if maybe_pulse_type.is_none() {
                                            trace!("Detected an invalid pulse");
                                            state.preamble_detected = false;
                                            state.bits.clear();
                                        }

                                        state.last_pulse = maybe_pulse_type;
                                    }
                                }
                                None => panic!("The Pin has been dropped"),
                            }
                        }
                        _ = _stop_rx.changed() => {
                            break;
                        }
                    }
                }
            });

            tasks.push(wireless_button_listener);

            let mut _msg_tx = msg_tx.clone();
            let mut _stop_rx = stop_rx.clone();
            let mut _settings_rx = settings_rx.clone();

            let button_listener = task::spawn(async move {
                let mut wired_pressed = false;
                let mut wireless_pressed = false;
                let mut wireless_expires = None;
                let mut sound = None;

                let mut was_pressed = false;
                let mut last_sound = None;

                loop {
                    let wireless_expiration = if let Some(time) = wireless_expires {
                        WirelessTimeout::Time(Box::pin(sleep_until(time)))
                    } else {
                        WirelessTimeout::Never(core::future::pending())
                    };

                    tokio::pin!(wireless_expiration);

                    tokio::select! {
                        level = wired_rx.recv() => {
                            match level {
                                Some(Level::Low) => wired_pressed = false,
                                Some(Level::High) => {
                                    wired_pressed = true;
                                    sound = None;
                                }
                                None => break,
                            }
                        }
                        remote = wireless_rx.recv() => {
                            match remote {
                                Some(id) => if let Some(rem) = settings.remotes.iter().find(|rem| rem.id == id) {
                                    wireless_pressed = true;
                                    wireless_expires = Some(Instant::now() + BUTTON_TIMEOUT);
                                    sound = rem.sound;
                                }
                                None => break,
                            }
                        }
                        maybe_err = _settings_rx.changed() => {
                            match maybe_err {
                                Ok(()) => {
                                    settings = _settings_rx.borrow().clone();
                                }
                                Err(_) => break,
                            }
                        }
                        _ = wireless_expiration => {
                            wireless_pressed = false;
                            wireless_expires = None;
                        }
                        _ = _stop_rx.changed() => break,
                    }

                    let pressed = wired_pressed || wireless_pressed;
                    if pressed != was_pressed || sound != last_sound {
                        _msg_tx
                            .send(if pressed {
                                SoundMessage::StartBuzzer(sound)
                            } else {
                                SoundMessage::StopBuzzer
                            })
                            .unwrap();
                        was_pressed = pressed;
                        last_sound = sound;
                    }
                }
            });

            tasks.push(button_listener);

            (Some((wired_pin, ant_pin)), Some(remote_id_rx))
        } else {
            (None, None)
        };

        Self {
            _context: context,
            msg_tx,
            settings_tx,
            stop_tx,
            tasks,
            #[cfg(target_os = "linux")]
            remote_id_rx,
            #[cfg(target_os = "linux")]
            _pins,
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

    /// Waits for a remote to be detected, then passes the id value to `callback`.
    /// If buttons are not available on the current system, `callback` will never
    /// be called.
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    pub fn request_next_remote_id<F>(&self, callback: F)
    where
        F: FnOnce(u32) + Send + 'static,
    {
        #[cfg(target_os = "linux")]
        if let Some(mut rx) = self.remote_id_rx.clone() {
            rx.borrow_and_update();
            task::spawn(async move {
                rx.changed().await.unwrap();
                callback(*rx.borrow());
            });
        }
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

#[cfg(target_os = "linux")]
const fn identify_pulse(len: u128) -> Option<PulseType> {
    const SHORT_PULSE_BOT_THRESH: u128 = 200;
    const SHORT_PULSE_TOP_THRESH: u128 = 500;
    const LONG_PULSE_BOT_THRESH: u128 = 800;
    const LONG_PULSE_TOP_THRESH: u128 = 1500;
    const PREAMBLE_PULSE_BOT_THRESH: u128 = 9000;
    const PREAMBLE_PULSE_TOP_THRESH: u128 = 12000;

    match len {
        SHORT_PULSE_BOT_THRESH..=SHORT_PULSE_TOP_THRESH => Some(PulseType::Short),
        LONG_PULSE_BOT_THRESH..=LONG_PULSE_TOP_THRESH => Some(PulseType::Long),
        PREAMBLE_PULSE_BOT_THRESH..=PREAMBLE_PULSE_TOP_THRESH => Some(PulseType::Preamble),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
struct RemoteDetectorState {
    preamble_detected: bool,
    bits: Vec<bool>,
    last_pin_state: Level,
    last_edge_time: Instant,
    last_pulse: Option<PulseType>,
}

#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PulseType {
    Short,
    Long,
    Preamble,
}

#[cfg(target_os = "linux")]
enum WirelessTimeout {
    Never(core::future::Pending<()>),
    Time(Pin<Box<tokio::time::Sleep>>),
}

#[cfg(target_os = "linux")]
impl Future for WirelessTimeout {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self {
            Self::Never(ref mut pend) => pend.poll(cx),
            Self::Time(ref mut slp) => slp.poll(cx),
        }
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
