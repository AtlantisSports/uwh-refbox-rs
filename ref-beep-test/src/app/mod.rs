use super::snapshot::{BeepTestPeriod, BeepTestSnapshot};
use crate::{config::Config, sound_controller::*, tournament_manager::*, APP_NAME};
use iced::{executor, widget::column, Application, Command, Subscription};
use iced_futures::{
    futures::stream::{self, BoxStream},
    subscription::{EventStream, Recipe},
};
use iced_runtime::command;
use log::*;
use message::{BoolGameParameter, CyclingParameter, Message};
use std::{
    borrow::Cow,
    process::Child,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, watch},
    time::{timeout_at, Instant},
};
use tokio_serial::SerialPortBuilder;
use update_sender::UpdateSender;

pub(crate) mod message;

pub(crate) mod update_sender;
mod view_builders;
use view_builders::*;

pub mod style;
use style::{PADDING, SPACING};

#[derive(Clone, Debug)]
struct TimeUpdater {
    tm: Arc<Mutex<TournamentManager>>,
    clock_running_receiver: watch::Receiver<bool>,
}

#[derive(Debug)]
pub struct RefBeepTestAppFlags {
    pub config: Config,
    pub serial_ports: Vec<SerialPortBuilder>,
    pub binary_port: u16,
    pub json_port: u16,
    pub sim_child: Option<Child>,
}

#[derive(Debug, Clone)]
enum AppState {
    MainPage,
    Settings,
}

pub struct BeepTestApp {
    config: Config,
    edited_settings: Option<EditableSettings>,
    tm: Arc<Mutex<TournamentManager>>,
    time_updater: TimeUpdater,
    snapshot: BeepTestSnapshot,
    sound: SoundController,
    sim_child: Option<Child>,
    last_message: Message,
    update_sender: UpdateSender,
    app_state: AppState,
}

pub type Element<'a, Message> = iced::Element<'a, Message, iced::Renderer<style::ApplicationTheme>>;

impl BeepTestApp {
    fn apply_snapshot(&mut self, new_snapshot: BeepTestSnapshot) {
        self.maybe_play_sound(&new_snapshot);
        self.update_sender
            .send_snapshot(new_snapshot.clone())
            .unwrap();
        self.snapshot = new_snapshot;
    }

    fn maybe_play_sound(&self, new_snapshot: &BeepTestSnapshot) {
        let (play_whistle, play_buzzer) = {
            let prereqs = new_snapshot.current_period != BeepTestPeriod::Pre
                && new_snapshot.secs_in_period != self.snapshot.secs_in_period;

            let is_whistle_period = match new_snapshot.current_period {
                BeepTestPeriod::Level(_) => true,
                BeepTestPeriod::Pre => false,
            };

            let (end_starts_play, end_stops_play) = (true, false);

            let is_buzz_period = end_starts_play && self.config.sound.auto_sound_start_play
                || end_stops_play && self.config.sound.auto_sound_stop_play;

            (
                prereqs && is_whistle_period && new_snapshot.secs_in_period == 5,
                prereqs && is_buzz_period && new_snapshot.secs_in_period == 0,
            )
        };

        if play_whistle {
            info!("Triggering whistle");
            self.sound.trigger_whistle();
        } else if play_buzzer {
            info!("Triggering buzzer");
            self.sound.trigger_buzzer();
        }
    }

    fn apply_settings_change(&mut self) {
        let edited_settings = self.edited_settings.take().unwrap();

        let EditableSettings { sound } = edited_settings;
        self.config.sound = sound;
        self.sound.update_settings(self.config.sound.clone());
    }
}

impl Drop for BeepTestApp {
    fn drop(&mut self) {
        if let Some(mut child) = self.sim_child.take() {
            info!("Waiting for child");
            child.wait().unwrap();
        }
    }
}

impl Application for BeepTestApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = style::ApplicationTheme;
    type Flags = RefBeepTestAppFlags;

    fn title(&self) -> String {
        "UWH Ref Beep Test".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        trace!("Handling message: {message:?}");

        if !message.is_repeatable() && (message == self.last_message) {
            warn!("Ignoring a repeated message: {message:?}");
            self.last_message = message.clone();
            return Command::none();
        } else {
            self.last_message = message.clone();
        }

        match message {
            Message::CycleParameter(param) => {
                let settings = &mut self.edited_settings.as_mut().unwrap();
                match param {
                    CyclingParameter::BuzzerSound => settings.sound.buzzer_sound.cycle(),
                    CyclingParameter::AlertVolume => settings.sound.whistle_vol.cycle(),
                    CyclingParameter::AboveWaterVol => settings.sound.above_water_vol.cycle(),
                    CyclingParameter::UnderWaterVol => settings.sound.under_water_vol.cycle(),
                }
            }
            Message::ToggleBoolParameter(param) => {
                dbg!(&self.edited_settings);
                let edited_settings = self.edited_settings.as_mut().unwrap();
                match param {
                    BoolGameParameter::SoundEnabled => edited_settings.sound.sound_enabled ^= true,
                    BoolGameParameter::RefAlertEnabled => {
                        edited_settings.sound.whistle_enabled ^= true
                    }
                }
            }
            Message::Reset => {
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                tm.reset_beep_test_now(now);
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot);
            }
            Message::Start => {
                let mut tm = self.tm.lock().unwrap();
                let now = Instant::now();
                match tm.current_period() {
                    BeepTestPeriod::Pre => tm.start_beep_test_now(now).unwrap(),
                    BeepTestPeriod::Level(_) => tm.start_clock(now),
                }
                let snapshot = tm.generate_snapshot(now).unwrap();
                std::mem::drop(tm);
                self.apply_snapshot(snapshot);
            }
            Message::Stop => self.tm.lock().unwrap().stop_clock(Instant::now()).unwrap(),

            Message::ShowSettings => {
                self.edited_settings = Some(EditableSettings {
                    sound: self.config.sound.clone(),
                });
                self.app_state = AppState::Settings;
                trace!("AppState changed to {:?}", self.app_state);
            }

            Message::NewSnapshot(snapshot) => {
                self.apply_snapshot(snapshot);
            }
            Message::EditComplete => {
                self.app_state = {
                    self.apply_settings_change();

                    confy::store(APP_NAME, None, &self.config).unwrap();
                    AppState::MainPage
                }
            }
            Message::NoAction => {}
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let clock_running = self.tm.lock().unwrap().clock_is_running();
        let main_view = column![match self.app_state {
            AppState::MainPage =>
                build_main_view(&self.snapshot, clock_running, &self.config.beep_test),
            AppState::Settings =>
                make_sound_config_page(&self.snapshot, self.edited_settings.as_ref().unwrap(),),
        }]
        .spacing(SPACING)
        .padding(PADDING);
        main_view.into()
    }

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let Self::Flags {
            config,
            serial_ports,
            binary_port,
            json_port,
            sim_child,
        } = flags;

        let tm = TournamentManager::new(config.beep_test.clone());
        tm.send_clock_running(false);

        let clock_running_receiver = tm.get_start_stop_rx();

        let tm = Arc::new(Mutex::new(tm));

        let update_sender = UpdateSender::new(serial_ports, binary_port, json_port);

        let sound =
            SoundController::new(config.sound.clone(), update_sender.get_trigger_flash_fn());

        let snapshot = Default::default();

        (
            Self {
                config,
                edited_settings: None,
                tm: tm.clone(),
                time_updater: TimeUpdater {
                    tm: tm.clone(),
                    clock_running_receiver,
                },
                last_message: Message::NoAction,
                snapshot,
                sound,
                update_sender,
                sim_child,
                app_state: AppState::MainPage,
            },
            Command::single(command::Action::LoadFont {
                bytes: Cow::from(&include_bytes!("../../resources/Roboto-Medium.ttf")[..]),
                tagger: Box::new(|res| match res {
                    Ok(()) => {
                        info!("Loaded font");
                        Message::NoAction
                    }
                    Err(e) => panic!("Failed to load font: {e:?}"),
                }),
            }),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([Subscription::from_recipe(self.time_updater.clone())])
    }
}

impl Recipe for TimeUpdater {
    type Output = Message;

    fn hash(&self, state: &mut iced_core::Hasher) {
        use std::hash::Hash;

        "TimeUpdater".hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        debug!("Updater started");

        struct State {
            tm: Arc<Mutex<TournamentManager>>,
            clock_running_receiver: watch::Receiver<bool>,
            next_time: Option<Instant>,
        }

        let state = State {
            tm: self.tm.clone(),
            clock_running_receiver: self.clock_running_receiver.clone(),
            next_time: Some(Instant::now()),
        };

        Box::pin(stream::unfold(state, |mut state| async move {
            let mut clock_running = true;
            if let Some(next_time) = state.next_time {
                if next_time > Instant::now() {
                    match timeout_at(next_time, state.clock_running_receiver.changed()).await {
                        Err(_) => {}
                        Ok(Err(_)) => return None,
                        Ok(Ok(())) => {
                            clock_running = *state.clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                    };
                } else {
                    match state.clock_running_receiver.has_changed() {
                        Ok(true) => {
                            clock_running = *state.clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                        Ok(false) => {}
                        Err(_) => {
                            return None;
                        }
                    };
                }
            } else {
                debug!("Awaiting a new clock running message");
                match state.clock_running_receiver.changed().await {
                    Err(_) => return None,
                    Ok(()) => {
                        clock_running = *state.clock_running_receiver.borrow();
                        debug!("Received clock running message: {clock_running}");
                    }
                };
            };

            let mut tm = state.tm.lock().unwrap();
            let now = Instant::now();

            tm.update(now).unwrap();

            let snapshot = match tm.generate_snapshot(now) {
                Some(val) => val,
                None => {
                    error!("Failed to generate snapshot. State:\n{tm:#?}");
                    panic!("No snapshot");
                }
            };

            state.next_time = if clock_running {
                Some(tm.next_update_time(now).unwrap())
            } else {
                None
            };

            drop(tm);

            Some((Message::NewSnapshot(snapshot), state))
        }))
    }
}

#[derive(Debug, Clone)]
struct MessageListener {
    rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Message>>>>,
}

impl Recipe for MessageListener {
    type Output = Message;

    fn hash(&self, state: &mut iced_core::Hasher) {
        use std::hash::Hash;

        "MessageListener".hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        info!("Message Listener started");

        let rx = self
            .rx
            .lock()
            .unwrap()
            .take()
            .expect("Listener has already been started");

        Box::pin(stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|msg| (msg, rx))
        }))
    }
}
