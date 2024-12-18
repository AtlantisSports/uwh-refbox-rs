use super::snapshot::{BeepTestPeriod, BeepTestSnapshot};
use crate::{APP_NAME, config::Config, sound_controller::*, tournament_manager::*};
use futures_lite::Stream;
use iced::{Element, Subscription, Task, Theme, application::Appearance, widget::column};
use log::*;
use message::{BoolGameParameter, CyclingParameter, Message};
use std::{
    process::Child,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::mpsc,
    time::{Instant, timeout_at},
};
use tokio_serial::SerialPortBuilder;
use update_sender::UpdateSender;

pub(crate) mod message;

pub(crate) mod update_sender;
mod view_builders;
use view_builders::*;

pub mod theme;
use theme::*;

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
    snapshot: BeepTestSnapshot,
    sound: SoundController,
    sim_child: Option<Child>,
    last_message: Message,
    update_sender: UpdateSender,
    app_state: AppState,
}

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

    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        trace!("Handling message: {message:?}");

        if !message.is_repeatable() && (message == self.last_message) {
            warn!("Ignoring a repeated message: {message:?}");
            self.last_message = message.clone();
            return Task::none();
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
            Message::TimeUpdaterStarted(tx) => {
                tx.blocking_send(self.tm.clone()).unwrap();
            }
            Message::NoAction => {}
        }
        Task::none()
    }

    pub(super) fn view(&self) -> Element<Message> {
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

    pub(super) fn new(flags: RefBeepTestAppFlags) -> (Self, Task<Message>) {
        let RefBeepTestAppFlags {
            config,
            serial_ports,
            binary_port,
            json_port,
            sim_child,
        } = flags;

        let tm = TournamentManager::new(config.beep_test.clone());
        tm.send_clock_running(false);

        let tm = Arc::new(Mutex::new(tm));

        let update_sender = UpdateSender::new(serial_ports, binary_port, json_port);

        let sound =
            SoundController::new(config.sound.clone(), update_sender.get_trigger_flash_fn());

        let snapshot = Default::default();

        (
            Self {
                config,
                edited_settings: None,
                tm,
                last_message: Message::NoAction,
                snapshot,
                sound,
                update_sender,
                sim_child,
                app_state: AppState::MainPage,
            },
            Task::none(),
        )
    }

    pub(super) fn subscription(&self) -> Subscription<Message> {
        Subscription::run(time_updater)
    }

    pub fn application_style(&self, _theme: &Theme) -> Appearance {
        Appearance {
            background_color: WINDOW_BACKGROUND,
            text_color: BLACK,
        }
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

fn time_updater() -> impl Stream<Item = Message> {
    use iced::futures::SinkExt;
    debug!("Updater started");

    iced::stream::channel(100, async |mut msg_tx| {
        let (tx, mut rx) = mpsc::channel(1);

        msg_tx.send(Message::TimeUpdaterStarted(tx)).await.unwrap();

        let tm = rx.recv().await.unwrap();
        let mut clock_running_receiver = tm.lock().unwrap().get_start_stop_rx();
        let mut next_time = Some(Instant::now());

        loop {
            let mut clock_running = true;
            if let Some(next_time) = next_time {
                if next_time > Instant::now() {
                    match timeout_at(next_time, clock_running_receiver.changed()).await {
                        Err(_) => {}
                        Ok(Err(_)) => continue,
                        Ok(Ok(())) => {
                            clock_running = *clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                    };
                } else {
                    match clock_running_receiver.has_changed() {
                        Ok(true) => {
                            clock_running = *clock_running_receiver.borrow();
                            debug!("Received clock running message: {clock_running}");
                        }
                        Ok(false) => {}
                        Err(_) => {
                            continue;
                        }
                    };
                }
            } else {
                debug!("Awaiting a new clock running message");
                match clock_running_receiver.changed().await {
                    Err(_) => continue,
                    Ok(()) => {
                        clock_running = *clock_running_receiver.borrow();
                        debug!("Received clock running message: {clock_running}");
                    }
                };
            };

            let snapshot = {
                let mut tm = tm.lock().unwrap();
                let now = Instant::now();

                tm.update(now).unwrap();

                let snapshot = match tm.generate_snapshot(now) {
                    Some(val) => val,
                    None => {
                        error!("Failed to generate snapshot. State:\n{tm:#?}");
                        panic!("No snapshot");
                    }
                };

                next_time = if clock_running {
                    Some(tm.next_update_time(now).unwrap())
                } else {
                    None
                };

                drop(tm);

                snapshot
            };

            msg_tx.send(Message::NewSnapshot(snapshot)).await.unwrap();
        }
    })
}
