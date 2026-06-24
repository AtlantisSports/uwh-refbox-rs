use super::fl;
#[cfg(target_os = "linux")]
use core::future::Future;
use derivative::Derivative;
use enum_derive_2018::EnumFromStr;
use log::*;
use macro_attr_2018::macro_attr;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Display,
    future::pending,
    pin::Pin,
    sync::Arc,
};
#[cfg(target_os = "linux")]
use tokio::sync::watch::Receiver;
use tokio::{
    sync::{
        mpsc::{UnboundedSender, unbounded_channel},
        watch::{self, Sender},
    },
    task::{self, AbortHandle, JoinError, JoinHandle, JoinSet},
    time::{Duration, Instant, sleep, sleep_until, timeout},
};
use toml::Table;
use web_audio_api::{
    AudioBuffer,
    context::{AudioContext, AudioContextOptions, BaseAudioContext},
    media_devices,
    node::{
        AudioBufferSourceNode, AudioNode, AudioScheduledSourceNode, ChannelInterpretation,
        ChannelMergerNode, GainNode,
    },
};

const FADE_LEN: f64 = 0.05;
const FADE_WAIT: Duration = Duration::from_nanos((FADE_LEN * 1_000_000_000.0) as u64);

// Target length for a timed (auto) buzzer. The actual play time is snapped to
// the nearest whole number of the chosen sound's loop cycles (see
// `whole_cycles_for`), so the buzzer always ends on a complete pattern rather
// than partway through one. A fixed length lands mid-pattern: 2.15s is 4.3
// Whoop cycles, which would chop off a partial 5th whoop.
const SOUND_LEN: f64 = 2.15;

#[cfg(target_os = "linux")]
const BUTTON_TIMEOUT: Duration = Duration::from_millis(600);

mod sounds;
pub use sounds::*;

mod button_handler;
pub use button_handler::RemoteId;
use button_handler::*;

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
    pub manual_alarm_enabled: bool,
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
            mut manual_alarm_enabled,
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
        if let Some(old_manual_alarm_enabled) = old.get("manual_alarm_enabled") {
            if let Some(old_manual_alarm_enabled) = old_manual_alarm_enabled.as_bool() {
                manual_alarm_enabled = old_manual_alarm_enabled;
            }
        }
        if let Some(old_remotes) = old.get("remotes") {
            if let Some(old_remotes) = old_remotes.as_array() {
                remotes = old_remotes
                    .iter()
                    .filter_map(|r| {
                        if let Some(r) = r.as_table() {
                            let id = (r.get("id")?.as_integer()? as u32).into();
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
            manual_alarm_enabled,
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
    pub id: RemoteId,
    pub sound: Option<BuzzerSound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SoundId {
    AutoBuzzer,
    TestBuzzer(BuzzerSound),
    Whistle,
    CountdownBeep,
    ManualAlarm,
    #[cfg(target_os = "linux")]
    WiredButton,
    #[cfg(target_os = "linux")]
    WirelessButton(RemoteId),
}

struct SoundEnds {
    join_set: JoinSet<SoundId>,
    handles: BTreeMap<SoundId, AbortHandle>,
}

impl SoundEnds {
    fn new() -> Self {
        Self {
            join_set: JoinSet::new(),
            handles: BTreeMap::new(),
        }
    }

    async fn join_next(&mut self) -> Result<SoundId, JoinError> {
        self.handles.retain(|_, handle| !handle.is_finished());

        if let Some(res) = self.join_set.join_next().await {
            res
        } else {
            pending().await
        }
    }

    /// Cancels the sound with the given `sound_id`. If the sound is
    /// not in the set, this function does nothing.
    fn cancel(&self, sound_id: &SoundId) {
        if let Some(handle) = self.handles.get(sound_id) {
            handle.abort();
        }
    }

    #[cfg(target_os = "linux")]
    fn contains(&self, sound_id: &SoundId) -> bool {
        self.handles.contains_key(sound_id)
    }

    fn add(&mut self, sound_id: SoundId, end: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let handle = self.join_set.spawn(async move {
            end.await;
            sound_id
        });
        self.handles.insert(sound_id, handle);
    }
}

pub struct SoundController {
    msg_tx: UnboundedSender<SoundMessage>,
    settings_tx: Sender<SoundSettings>,
    stop_tx: Sender<bool>,
    handle: Option<JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    _button_handler: Option<ButtonHandler>,
    #[cfg(target_os = "linux")]
    remote_id_rx: Option<Receiver<RemoteId>>,
}

impl SoundController {
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    pub fn new<F>(mut settings: SoundSettings, trigger_flash: F) -> Self
    where
        F: Send
            + Sync
            + Fn() -> Result<(), tokio::sync::mpsc::error::TrySendError<ServerMessage>>
            + 'static,
    {
        let available_devices = media_devices::enumerate_devices_sync();

        debug!("Available audio devices:\n{:#?}", available_devices);

        let opts = AudioContextOptions {
            sample_rate: Some(SAMPLE_RATE),
            ..AudioContextOptions::default()
        };

        let context = AudioContext::new(opts);
        debug!("Audio context created with sink {:?}", context.sink_id());

        let context = Arc::new(context);

        let library = SoundLibrary::new(&context);

        let (msg_tx, mut msg_rx) = unbounded_channel();

        let (settings_tx, mut settings_rx) = watch::channel(settings.clone());
        settings_rx.borrow_and_update();

        let (stop_tx, mut stop_rx) = watch::channel(false);
        stop_rx.borrow_and_update();

        let mut _stop_rx = stop_rx.clone();
        let mut _settings_rx = settings_rx.clone();

        let handle = task::spawn(async move {
            // Owned by the worker so the audio output device can be rebuilt in
            // place when the operator presses "UPDATE AUDIO OUTPUT". A fresh
            // AudioContext resolves the CURRENT system default output device.
            let mut context = context;
            let mut library = library;
            #[cfg_attr(not(target_os = "linux"), allow(unused_assignments))]
            let mut last_sound: Option<(SoundId, Sound)> = None;
            let mut sound_queue: VecDeque<SoundId> = VecDeque::new();
            let mut sound_ends: SoundEnds = SoundEnds::new();

            loop {
                tokio::select! {
                    msg = msg_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                match msg {
                                    SoundMessage::TriggerBuzzer => {
                                        if !sound_queue.contains(&SoundId::AutoBuzzer) {
                                            sound_queue.push_back(SoundId::AutoBuzzer);
                                        }
                                    }
                                    SoundMessage::TriggerWhistle => {
                                        if !sound_queue.contains(&SoundId::Whistle) {
                                            sound_queue.push_back(SoundId::Whistle);
                                        }
                                    }
                                    SoundMessage::TriggerCountdownBeep => {
                                        if !sound_queue.contains(&SoundId::CountdownBeep) {
                                            sound_queue.push_back(SoundId::CountdownBeep);
                                        }
                                    }
                                    SoundMessage::StartManualBuzzer => {
                                        if !sound_queue.contains(&SoundId::ManualAlarm) {
                                            // Push to front so the manual alarm immediately
                                            // takes priority over any queued auto-buzzer.
                                            sound_queue.push_front(SoundId::ManualAlarm);
                                        }
                                    }
                                    SoundMessage::StopManualBuzzer => {
                                        if sound_queue.contains(&SoundId::ManualAlarm) {
                                            sound_queue.retain(|s| *s != SoundId::ManualAlarm);
                                        }
                                    }
                                    SoundMessage::TestBuzzer(sound) => {
                                        let id = SoundId::TestBuzzer(sound);
                                        if !sound_queue.contains(&id) {
                                            sound_queue.push_back(id);
                                        }
                                    }
                                    SoundMessage::ReloadAudioOutput => {
                                        info!("Reloading audio output to current system default");
                                        // Stop any sound playing on the OLD device first.
                                        if let Some((sound_id, sound)) = last_sound.take() {
                                            sound.stop().await;
                                            sound_ends.cancel(&sound_id);
                                        }
                                        // A fresh context (default sink) re-resolves
                                        // the device the OS currently has as default.
                                        let opts = AudioContextOptions {
                                            sample_rate: Some(SAMPLE_RATE),
                                            ..AudioContextOptions::default()
                                        };
                                        let new_context = AudioContext::new(opts);
                                        debug!(
                                            "Audio output reloaded with sink {:?}",
                                            new_context.sink_id()
                                        );
                                        context = Arc::new(new_context);
                                        library = SoundLibrary::new(&context);
                                        // sound_queue holds only SoundId values; they are
                                        // realized into Sound objects later by start_sound,
                                        // which uses the new context — so the queue is
                                        // intentionally left intact here, not drained.
                                    }
                                    #[cfg(target_os = "linux")]
                                    SoundMessage::StartWiredBuzzer => {
                                        if !sound_queue.contains(&SoundId::WiredButton) {
                                            sound_queue.push_back(SoundId::WiredButton);
                                        }
                                    }
                                    #[cfg(target_os = "linux")]
                                    SoundMessage::StopWiredBuzzer => {
                                        if sound_queue.contains(&SoundId::WiredButton) {
                                            sound_queue.retain(|s| *s != SoundId::WiredButton);
                                        }
                                    }
                                    #[cfg(target_os = "linux")]
                                    SoundMessage::WirelessRemoteReceived(id) => {
                                        if settings.remotes.iter().any(|r| r.id == id) {
                                            if !sound_queue.contains(&SoundId::WirelessButton(id)) {
                                                sound_queue.push_back(SoundId::WirelessButton(id));}
                                            if sound_ends.contains(&SoundId::WirelessButton(id)) {
                                                sound_ends.cancel(&SoundId::WirelessButton(id));
                                            }
                                            sound_ends.add(SoundId::WirelessButton(id), Box::pin(sleep(BUTTON_TIMEOUT)));
                                        }
                                    }
                                }
                            },
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
                    ended_sound = sound_ends.join_next() => {
                        match ended_sound {
                            Ok(sound_id) => {
                                if sound_queue.contains(&sound_id) {
                                    sound_queue.retain(|s| *s != sound_id);
                                }
                            }
                            Err(e) => {
                                if !e.is_cancelled() {
                                    error!("A sound end task failed: {e}");
                                }
                            }
                        }
                    }
                    _ = _stop_rx.changed() => {
                        break;
                    }
                }

                let start_sound = |last_sound: &mut Option<(SoundId, Sound)>,
                                   sound_ends: &mut SoundEnds,
                                   sound_id,
                                   flash| {
                    let sound = match sound_id {
                        SoundId::AutoBuzzer => {
                            info!("Auto-triggering buzzer");
                            let volumes = ChannelVolumes::new(&settings, false);
                            if flash {
                                trigger_flash().unwrap();
                            }
                            Sound::new(
                                context.clone(),
                                volumes,
                                library[settings.buzzer_sound].clone(),
                                true,
                                true,
                            )
                        }
                        SoundId::TestBuzzer(sound) => {
                            info!("Testing buzzer sound {sound:?}");
                            let volumes = ChannelVolumes::new(&settings, false);
                            Sound::new(context.clone(), volumes, library[sound].clone(), true, true)
                        }
                        SoundId::Whistle => {
                            info!("Playing whistle once");
                            let volumes = ChannelVolumes::new(&settings, true);
                            Sound::new(
                                context.clone(),
                                volumes,
                                library.whistle().clone(),
                                false,
                                false,
                            )
                        }
                        SoundId::CountdownBeep => {
                            info!("Playing countdown beep once");
                            let volumes = ChannelVolumes::new(&settings, false);
                            Sound::new(
                                context.clone(),
                                volumes,
                                library.countdown().clone(),
                                false,
                                false,
                            )
                        }
                        SoundId::ManualAlarm => {
                            info!("Manual alarm buzzer started");
                            let volumes = ChannelVolumes::new(&settings, false);
                            if flash {
                                trigger_flash().unwrap();
                            }
                            Sound::new(
                                context.clone(),
                                volumes,
                                library[settings.buzzer_sound].clone(),
                                true,
                                false,
                            )
                        }
                        #[cfg(target_os = "linux")]
                        SoundId::WiredButton => {
                            info!("Starting wired buzzer");
                            let volumes = ChannelVolumes::new(&settings, false);
                            if flash {
                                trigger_flash().unwrap();
                            }
                            Sound::new(
                                context.clone(),
                                volumes,
                                library[settings.buzzer_sound].clone(),
                                true,
                                false,
                            )
                        }
                        #[cfg(target_os = "linux")]
                        SoundId::WirelessButton(id) => {
                            let volumes = ChannelVolumes::new(&settings, false);
                            if let Some(buzzer_sound) = settings
                                .remotes
                                .iter()
                                .find(|r| r.id == id)
                                .map(|r| r.sound.unwrap_or(settings.buzzer_sound))
                            {
                                info!("Starting buzzer sound {buzzer_sound:?} for remote {id}");
                                if flash {
                                    trigger_flash().unwrap();
                                }
                                Sound::new(
                                    context.clone(),
                                    volumes,
                                    library[buzzer_sound].clone(),
                                    true,
                                    false,
                                )
                            } else {
                                error!("No buzzer sound found for remote {id}");
                                return;
                            }
                        }
                    };

                    if let Some(end) = sound.sound_end() {
                        sound_ends.add(sound_id, Box::pin(end));
                    }
                    *last_sound = Some((sound_id, sound));
                };

                match (last_sound.is_some(), sound_queue.is_empty()) {
                    (true, true) => {
                        if let Some((sound_id, sound)) = last_sound.take() {
                            info!("Stopping sound: {sound_id:?}");
                            sound.stop().await;
                            sound_ends.cancel(&sound_id);
                        }
                    }
                    (false, false) => {
                        start_sound(&mut last_sound, &mut sound_ends, sound_queue[0], true);
                    }
                    (false, true) => {}
                    (true, false) => {
                        if let Some((last_sound_id, _)) = last_sound {
                            if last_sound_id != sound_queue[0] {
                                if let Some((sound_id, sound)) = last_sound.take() {
                                    info!("Stopping sound: {sound_id:?}");
                                    sound.stop().await;
                                    sound_ends.cancel(&sound_id);
                                }
                                let flash = last_sound_id == SoundId::Whistle;
                                start_sound(
                                    &mut last_sound,
                                    &mut sound_ends,
                                    sound_queue[0],
                                    flash,
                                );
                            }
                        }
                    }
                }
            }
        });

        #[cfg(target_os = "linux")]
        let (remote_id_tx, remote_id_rx) = watch::channel(0.into());

        #[cfg(target_os = "linux")]
        let _button_handler = ButtonHandler::new(msg_tx.clone(), remote_id_tx);

        #[cfg(target_os = "linux")]
        let remote_id_rx = if _button_handler.is_some() {
            Some(remote_id_rx)
        } else {
            None
        };

        Self {
            msg_tx,
            settings_tx,
            stop_tx,
            handle: Some(handle),
            #[cfg(target_os = "linux")]
            _button_handler,
            #[cfg(target_os = "linux")]
            remote_id_rx,
        }
    }

    pub fn update_settings(&self, settings: SoundSettings) {
        self.settings_tx.send(settings).unwrap()
    }

    pub fn trigger_whistle(&self) {
        self.msg_tx.send(SoundMessage::TriggerWhistle).unwrap()
    }

    pub fn trigger_countdown_beep(&self) {
        self.msg_tx
            .send(SoundMessage::TriggerCountdownBeep)
            .unwrap()
    }

    pub fn start_manual_buzzer(&self) {
        self.msg_tx.send(SoundMessage::StartManualBuzzer).unwrap()
    }

    pub fn stop_manual_buzzer(&self) {
        self.msg_tx.send(SoundMessage::StopManualBuzzer).unwrap()
    }

    pub fn trigger_buzzer(&self) {
        self.msg_tx.send(SoundMessage::TriggerBuzzer).unwrap()
    }

    // Called by the buzzer-picker Test button (wired in Task 5/6 of this feature).
    // The allow suppresses the dead-code lint that a bin crate emits for public
    // methods not yet called anywhere in the binary.
    #[allow(dead_code)]
    pub fn test_buzzer(&self, sound: BuzzerSound) {
        // The worker receiver lives for the app's lifetime; send only fails
        // after shutdown, when there is nothing left to play through anyway.
        self.msg_tx.send(SoundMessage::TestBuzzer(sound)).unwrap()
    }

    pub fn reload_audio_output(&self) {
        // The worker receiver lives for the app's lifetime; send only fails
        // after shutdown, when there is nothing left to play through anyway.
        self.msg_tx.send(SoundMessage::ReloadAudioOutput).unwrap()
    }

    /// Returns a future that resolves to the next detected remote id.
    /// If buttons are not available on the current system, the future
    /// will immediately resolve to `None`.
    pub fn request_next_remote_id(&self) -> impl Future<Output = Option<RemoteId>> + Send + use<> {
        #[cfg(target_os = "linux")]
        let remote_id_rx = self.remote_id_rx.clone();
        async move {
            #[cfg(target_os = "linux")]
            if let Some(mut rx) = remote_id_rx {
                rx.borrow_and_update();
                if rx.changed().await.is_ok() {
                    return Some(*rx.borrow());
                }
            }
            None
        }
    }
}

/// Maximum time the sound-controller teardown waits for its worker to finish
/// before proceeding with shutdown/restart anyway. A hung audio shutdown must
/// never be able to prevent the app from relaunching.
/// 🔧 PI: confirm on the spare Pi during the 5×-restart test.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Await the worker `JoinHandle`, but never longer than `limit`. On timeout,
/// log and return so teardown (and the pending relaunch) can proceed.
async fn await_handle_bounded(handle: JoinHandle<()>, limit: Duration) {
    match timeout(limit, handle).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("Sound controller thread failed: {e}"),
        Err(_) => warn!("Sound controller shutdown timed out after {limit:?}; proceeding"),
    }
}

impl Drop for SoundController {
    fn drop(&mut self) {
        if self.stop_tx.send(true).is_err() {
            return;
        }

        if let Some(handle) = self.handle.take() {
            tokio::runtime::Handle::current()
                .block_on(await_handle_bounded(handle, SHUTDOWN_TIMEOUT));
        }
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

/// Whole number of complete loop cycles to play so a timed (auto) buzzer ends
/// on a pattern boundary nearest the `target` length, never partway through a
/// pattern. Always at least one cycle; guards against a non-positive period.
fn whole_cycles_for(loop_period: f64, target: f64) -> u32 {
    if loop_period <= 0.0 {
        return 1;
    }
    ((target / loop_period).round() as i64).max(1) as u32
}

struct Sound {
    _merger: ChannelMergerNode,
    gain_l: GainNode,
    gain_r: GainNode,
    source: AudioBufferSourceNode,
    context: Arc<AudioContext>,
    volumes: ChannelVolumes,
    end: Option<Instant>,
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

        let buffer_secs = buffer.duration();
        let mut source = context.create_buffer_source();
        source.set_buffer(buffer);
        source.connect(&gain_l);
        source.connect(&gain_r);
        source.set_loop(repeat);

        let fade_end = context.current_time() + FADE_LEN;
        let start = Instant::now();

        // Set the gains so that the start of the fade is now
        gain_l.gain().set_value(0.0);
        gain_r.gain().set_value(0.0);

        gain_l
            .gain()
            .linear_ramp_to_value_at_time(volumes.left, fade_end);
        gain_r
            .gain()
            .linear_ramp_to_value_at_time(volumes.right, fade_end);

        let end = if timed {
            // The buffer loops from playback start (t0 ≈ current_time()). Play a
            // whole number of complete loop cycles so the buzzer ends on a
            // pattern boundary, and place the fade-out in the final cycle so it
            // completes exactly at that boundary — landing in the clip's trailing
            // silence for clips that have one.
            // Use the clip's OWN duration. The clip is 44.1 kHz, but the audio
            // context can run at another rate (commonly 48 kHz); the clip is
            // resampled on playback yet its real-time loop length stays
            // `buffer.duration()`. Deriving the period from `context.sample_rate()`
            // miscounts the cycles and lands the fade-out mid-pattern.
            let loop_period = buffer_secs;
            let played = whole_cycles_for(loop_period, SOUND_LEN) as f64 * loop_period;

            let t0 = fade_end - FADE_LEN;
            let fade_out_end = t0 + played;
            let fade_out_start = fade_out_end - FADE_LEN;

            gain_l
                .gain()
                .set_value_at_time(volumes.left, fade_out_start);
            gain_l
                .gain()
                .linear_ramp_to_value_at_time(0.0, fade_out_end);

            gain_r
                .gain()
                .set_value_at_time(volumes.right, fade_out_start);
            gain_r
                .gain()
                .linear_ramp_to_value_at_time(0.0, fade_out_end);

            Duration::try_from_secs_f64(played).ok().map(|d| start + d)
        } else if !repeat {
            // Same fix as above: a one-shot sound (the whistle) plays for its
            // own duration, independent of the context's sample rate.
            Duration::try_from_secs_f64(buffer_secs)
                .ok()
                .map(|d| start + d)
        } else {
            None
        };

        source.start();

        Self {
            _merger,
            gain_l,
            gain_r,
            source,
            context,
            volumes,
            end,
        }
    }

    /// If the sound has a predictable end time, this will return a future that resolves
    /// after the sound ends.
    fn sound_end(&self) -> Option<impl Future<Output = ()> + use<>> {
        self.end.map(|end| async move {
            sleep_until(end).await;
        })
    }

    async fn stop(mut self) {
        // Timed sounds schedule their own fade-out via Web Audio API automation.
        // If that scheduled end time has already passed, the gain is already at
        // zero — resetting it to full volume here would cause a brief burst of
        // audio (heard as a "tap" or click at the end of the buzzer). Only run
        // the fade-out when the sound is being stopped early (interrupted).
        let already_silent = self.end.is_some_and(|end| Instant::now() >= end);

        if !already_silent {
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
        }

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
        assert!(!settings.manual_alarm_enabled);
        assert_eq!(
            settings.remotes,
            vec![
                RemoteInfo {
                    id: 1.into(),
                    sound: Some(BuzzerSound::Buzz),
                },
                RemoteInfo {
                    id: 2.into(),
                    sound: Some(BuzzerSound::DeDeDu),
                },
            ]
        );
    }

    #[tokio::test]
    async fn await_handle_bounded_returns_for_hung_task() {
        // A worker that never finishes must not block teardown past the bound.
        let handle = tokio::spawn(async { std::future::pending::<()>().await });
        let start = Instant::now();
        await_handle_bounded(handle, Duration::from_millis(150)).await;
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "bounded shutdown must return shortly after the timeout"
        );
    }

    #[tokio::test]
    async fn await_handle_bounded_returns_promptly_for_finished_task() {
        let handle = tokio::spawn(async {});
        let start = Instant::now();
        await_handle_bounded(handle, Duration::from_secs(5)).await;
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn whole_cycles_rounds_to_nearest_whole_cycle() {
        // Target ~2.15s snapped to each bundled clip's loop period. These are
        // the counts that make the auto buzzer end on a complete pattern.
        assert_eq!(whole_cycles_for(0.5, 2.15), 4); // Whoop / Buzz (0.500s)
        assert_eq!(whole_cycles_for(29430.0 / 44100.0, 2.15), 3); // Crazy (0.667s)
        assert_eq!(whole_cycles_for(0.75, 2.15), 3); // De-De-Du (0.750s)
        assert_eq!(whole_cycles_for(0.8, 2.15), 3); // Two-Tone (0.800s)
        assert_eq!(whole_cycles_for(0.14, 2.15), 15); // Pip
        assert_eq!(whole_cycles_for(0.215, 2.15), 10); // Pipes
        assert_eq!(whole_cycles_for(0.70, 2.15), 3); // Airhorn/Klaxon/Pulse/Siren
        assert_eq!(whole_cycles_for(0.50, 2.15), 4); // Trill
    }

    #[test]
    fn whole_cycles_is_at_least_one() {
        // A clip longer than the target still plays one full cycle.
        assert_eq!(whole_cycles_for(3.0, 2.15), 1);
    }

    #[test]
    fn whole_cycles_guards_nonpositive_period() {
        assert_eq!(whole_cycles_for(0.0, 2.15), 1);
        assert_eq!(whole_cycles_for(-1.0, 2.15), 1);
    }

    /// The timed buzzer must fade out in the clip's trailing silence even when
    /// the audio context runs at a different sample rate than the 44.1 kHz
    /// clips (48 kHz is common on Windows/macOS). The loop length must come
    /// from the clip's own duration, not `context.sample_rate()`. Renders a
    /// synthetic "loud then silent" clip at 48 kHz and proves the fade lands in
    /// silence with the correct period, and in the loud region with the buggy
    /// `length / context_rate` period (so this test has teeth).
    #[test]
    fn timed_buzzer_fade_lands_in_clip_silence_at_48k_context() {
        use web_audio_api::context::OfflineAudioContext;

        let dev_sr = 48_000.0_f32; // context rate differs from the clip rate
        let clip_sr = 44_100.0_f32;
        let clip_len = 22_050_usize; // 0.5 s clip: loud 0..0.4 s, silent 0.4..0.5 s

        let render_tail_peak = |loop_period: f64| -> f32 {
            let render_secs = 2.7;
            let mut ctx =
                OfflineAudioContext::new(1, (dev_sr as f64 * render_secs) as usize, dev_sr);

            let mut buffer = ctx.create_buffer(1, clip_len, clip_sr);
            let data: Vec<f32> = (0..clip_len)
                .map(|i| if (i as f32 / clip_sr) < 0.4 { 1.0 } else { 0.0 })
                .collect();
            buffer.copy_to_channel(&data, 0);
            assert!((buffer.duration() - 0.5).abs() < 1e-9);

            let gain = ctx.create_gain();
            gain.connect(&ctx.destination());
            let mut source = ctx.create_buffer_source();
            source.set_buffer(buffer);
            source.set_loop(true);
            source.connect(&gain);

            let played = whole_cycles_for(loop_period, SOUND_LEN) as f64 * loop_period;
            let fade_out_end = played;
            let fade_out_start = fade_out_end - FADE_LEN;
            gain.gain().set_value(0.0);
            gain.gain().linear_ramp_to_value_at_time(1.0, FADE_LEN);
            gain.gain().set_value_at_time(1.0, fade_out_start);
            gain.gain().linear_ramp_to_value_at_time(0.0, fade_out_end);
            source.start();

            let rendered = ctx.start_rendering_sync();
            let ch = rendered.get_channel_data(0);
            // Peak over the last 40 ms before the buzzer ends.
            let s = ((fade_out_end - 0.04) * dev_sr as f64) as usize;
            let e = ((fade_out_end * dev_sr as f64) as usize).min(ch.len());
            ch[s..e].iter().fold(0.0f32, |m, &x| m.max(x.abs()))
        };

        // FIXED: period = clip's own duration (0.5 s) -> ends in silence.
        let fixed_peak = render_tail_peak(0.5);
        assert!(
            fixed_peak < 1e-3,
            "fix: tail should be silent, peak={fixed_peak}"
        );

        // BUGGY: period = length / context_rate (22050/48000 = 0.459 s) ->
        // fade lands in the loud region. Documents the regression.
        let buggy_peak = render_tail_peak(clip_len as f64 / dev_sr as f64);
        assert!(
            buggy_peak > 0.5,
            "buggy period should land in the loud region, peak={buggy_peak}"
        );
    }
}
