#[cfg(target_os = "linux")]
use arrayref::array_ref;
#[cfg(target_os = "linux")]
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use derivative::Derivative;
use enum_derive_2018::EnumDisplay;
#[cfg(target_os = "linux")]
use futures_lite::future::FutureExt;
use log::*;
use macro_attr_2018::macro_attr;
use rodio::{
    decoder::DecoderError,
    source::{Buffered, ChannelVolume},
    Decoder, OutputStream, Sink, Source,
};
#[cfg(target_os = "linux")]
use rppal::gpio::{Gpio, InputPin, Level, Trigger};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, ops::Index};
#[cfg(target_os = "linux")]
use tokio::time::{sleep_until, Duration, Instant};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        watch::{self, Sender},
    },
    task::{self, JoinHandle},
};

#[cfg(target_os = "linux")]
const MESSAGE_LEN: usize = 24;
#[cfg(target_os = "linux")]
const ID_LEN: usize = 20;
#[cfg(target_os = "linux")]
const DATA_LEN: usize = MESSAGE_LEN - ID_LEN;

#[cfg(target_os = "linux")]
const BUTTON_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SoundSettings {
    #[derivative(Default(value = "true"))]
    pub sound_enabled: bool,
    #[derivative(Default(value = "true"))]
    pub ref_warn_enabled: bool,
    pub buzzer_sound: BuzzerSound,
    #[derivative(Default(value = "Volume::Medium"))]
    pub ref_warn_vol: Volume,
    pub above_water_vol: Volume,
    pub under_water_vol: Volume,
    pub remotes: Vec<RemoteInfo>,
}

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!)]
    #[derivative(Default)]
    pub enum BuzzerSound {
        #[derivative(Default)]
        Buzz,
        Tweedle,
    }
}

macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!)]
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
    fn to_f32(&self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 0.75,
            Self::Max => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub id: u32,
    pub sound: Option<BuzzerSound>,
}

type SoundType = ChannelVolume<Buffered<Decoder<Cursor<Vec<u8>>>>>;

struct SoundLibrary {
    buzz: SoundType,
    tweedle: SoundType,
    ref_warn: SoundType,
}

impl Index<BuzzerSound> for SoundLibrary {
    type Output = SoundType;

    fn index(&self, sound: BuzzerSound) -> &Self::Output {
        match sound {
            BuzzerSound::Buzz => &self.buzz,
            BuzzerSound::Tweedle => &self.tweedle,
        }
    }
}

impl SoundLibrary {
    fn new() -> Result<Self, DecoderError> {
        Ok(Self {
            buzz: ChannelVolume::new(
                Decoder::new_wav(Cursor::new(Vec::from(
                    include_bytes!("../resources/1000Hz-Both.wav").clone(),
                )))?
                .buffered(),
                vec![0., 0.],
            ),
            tweedle: ChannelVolume::new(
                Decoder::new_wav(Cursor::new(Vec::from(
                    include_bytes!("../resources/1000Hz-Both.wav").clone(),
                )))?
                .buffered(),
                vec![0., 0.],
            ),
            ref_warn: ChannelVolume::new(
                Decoder::new_wav(Cursor::new(Vec::from(
                    include_bytes!("../resources/1000Hz-Both.wav").clone(),
                )))?
                .buffered(),
                vec![0., 0.],
            ),
        })
    }

    fn set_volumes(&mut self, settings: &SoundSettings) {
        let left_vol = settings.above_water_vol.to_f32();
        let right_vol = settings.under_water_vol.to_f32();
        let ref_warn_vol = settings.ref_warn_vol.to_f32();

        self.buzz.set_volume(1, left_vol);
        self.buzz.set_volume(0, right_vol);
        self.tweedle.set_volume(1, left_vol);
        self.tweedle.set_volume(0, right_vol);
        self.ref_warn.set_volume(1, ref_warn_vol);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SoundMessage {
    TriggerBuzzer,
    TriggerRefWarning,
    #[cfg(target_os = "linux")]
    StartBuzzer,
    #[cfg(target_os = "linux")]
    StopBuzzer,
}

pub struct SoundController {
    _out_stream: OutputStream,
    msg_tx: UnboundedSender<SoundMessage>,
    settings_tx: Sender<SoundSettings>,
    stop_tx: Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    _pins: Option<(InputPin, InputPin)>,
}

impl SoundController {
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    pub fn new(mut settings: SoundSettings) -> Self {
        let mut library = SoundLibrary::new().unwrap();
        library.set_volumes(&settings);

        let (msg_tx, mut msg_rx) = unbounded_channel();

        let (settings_tx, mut settings_rx) = watch::channel(settings.clone());
        settings_rx.borrow_and_update();

        let (stop_tx, mut stop_rx) = watch::channel(false);
        stop_rx.borrow_and_update();

        let (_out_stream, handle) = OutputStream::try_default().unwrap();

        let mut _stop_rx = stop_rx.clone();
        let mut _settings_rx = settings_rx.clone();
        let mut _settings = settings.clone();

        let handler = task::spawn(async move {
            #[cfg_attr(not(target_os = "linux"), allow(unused_assignments))]
            let mut sink = Sink::try_new(&handle).unwrap();

            loop {
                tokio::select! {
                    msg = msg_rx.recv() => {
                        match msg {
                            Some(SoundMessage::TriggerBuzzer) => {
                                info!("Playing buzzer once");
                                let sound = library[_settings.buzzer_sound].clone();
                                sink = Sink::try_new(&handle).unwrap();
                                sink.append(sound);
                            }
                            Some(SoundMessage::TriggerRefWarning) => {
                                info!("Playing ref warning once");
                                let sound = library.ref_warn.clone();
                                sink = Sink::try_new(&handle).unwrap();
                                sink.append(sound);
                            }
                            #[cfg(target_os = "linux")]
                            Some(SoundMessage::StartBuzzer) => {
                                info!("Starting buzzer");
                                let sound = library[_settings.buzzer_sound].clone().repeat_infinite();
                                sink = Sink::try_new(&handle).unwrap();
                                sink.append(sound);
                            }
                            #[cfg(target_os = "linux")]
                            Some(SoundMessage::StopBuzzer) => {
                                info!("Stopping buzzer");
                                sink.stop();
                            },
                            None => break,
                        }
                    }
                    maybe_err = _settings_rx.changed() => {
                        match maybe_err {
                            Ok(()) => {
                                _settings = _settings_rx.borrow().clone();
                                library.set_volumes(&_settings);
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
        let _pins = if let Ok(sys_info) = rppal::system::DeviceInfo::new() {
            info!("Detected a Raspberry Pi system: {sys_info:?}, starting GPIO processes");

            let gpio = Gpio::new().unwrap();

            let mut ant_pin = gpio.get(16).unwrap().into_input_pullup();
            let (ant_tx, mut ant_rx) = unbounded_channel();
            ant_pin
                .set_async_interrupt(Trigger::Both, move |level| {
                    ant_tx.send((level, Instant::now())).unwrap()
                })
                .unwrap();

            let mut wired_pin = gpio.get(12).unwrap().into_input();
            let (wired_tx, mut wired_rx) = unbounded_channel();
            wired_pin
                .set_async_interrupt(Trigger::Both, move |level| wired_tx.send(level).unwrap())
                .unwrap();

            let ant_start_state = ant_pin.read();

            let (wireless_tx, mut wireless_rx) = unbounded_channel();

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

                let mut was_pressed = false;

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
                                Some(Level::High) => wired_pressed = false,
                                Some(Level::Low) => wired_pressed = true,
                                None => break,
                            }
                        }
                        remote = wireless_rx.recv() => {
                            match remote {
                                Some(id) => if settings.remotes.iter().find(|rem| rem.id == id).is_some() {
                                    wireless_pressed = true;
                                    wireless_expires = Some(Instant::now() + BUTTON_TIMEOUT);
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
                    if pressed != was_pressed {
                        _msg_tx
                            .send(if pressed {
                                SoundMessage::StartBuzzer
                            } else {
                                SoundMessage::StopBuzzer
                            })
                            .unwrap();
                        was_pressed = pressed;
                    }
                }
            });

            tasks.push(button_listener);

            Some((wired_pin, ant_pin))
        } else {
            None
        };

        Self {
            _out_stream,
            msg_tx,
            settings_tx,
            stop_tx,
            tasks,
            #[cfg(target_os = "linux")]
            _pins,
        }
    }

    pub fn update_settings(&self, settings: SoundSettings) {
        self.settings_tx.send(settings).unwrap()
    }

    pub fn trigger_ref_warn(&self) {
        self.msg_tx.send(SoundMessage::TriggerRefWarning).unwrap()
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
