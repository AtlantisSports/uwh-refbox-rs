use crate::tournament_manager::*;
use eframe::{
    egui,
    epi::{self, Frame, Storage},
};
use log::*;
use std::{
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use uwh_common::{config::Game, game_snapshot::GameSnapshot};

#[derive(Debug)]
pub struct RefBoxApp {
    tm: Arc<Mutex<TournamentManager>>,
    updater_handle: JoinHandle<()>,
    frame_sender: Sender<Frame>,
    state_recv: Receiver<GameSnapshot>,
    snapshot: GameSnapshot,
}

impl RefBoxApp {
    pub fn new(config: Game) -> Self {
        let mut tm = TournamentManager::new(config);
        tm.start_clock(Instant::now());

        let (clock_running_send, clock_running_recv) = mpsc::channel();
        tm.add_start_stop_sender(clock_running_send);

        let tm = Arc::new(Mutex::new(tm));
        let tm_ = tm.clone();

        let (state_send, state_recv) = mpsc::channel();
        let (frame_sender, frame_recv): (Sender<Frame>, Receiver<Frame>) = mpsc::channel();

        let updater_handle = thread::spawn(move || {
            debug!("Updater started");
            let mut timeout = Duration::from_secs(1);
            let frame = frame_recv.recv().unwrap();

            let update_and_send_snapshot = move |tm: &mut MutexGuard<TournamentManager>| {
                let now = Instant::now();
                tm.update(now).unwrap();
                if let Some(snapshot) = tm.generate_snapshot(now) {
                    trace!("Updater: sending snapshot");
                    state_send.send(snapshot).unwrap();
                    frame.request_repaint();
                } else {
                    panic!("Failed to generate snapshot");
                }
                now
            };

            loop {
                match clock_running_recv.recv_timeout(timeout) {
                    Ok(false) => loop {
                        trace!("Updater: locking tm");
                        update_and_send_snapshot(&mut tm_.lock().unwrap());
                        info!("Updater: Waiting for Clock to start");
                        if clock_running_recv.recv().unwrap() {
                            info!("Updater: Clock has restarted");
                            timeout = Duration::from_secs(0);
                            break;
                        }
                    },
                    Err(RecvTimeoutError::Disconnected) => break,
                    Ok(true) | Err(RecvTimeoutError::Timeout) => {
                        trace!("Updater: locking tm");
                        let mut tm = tm_.lock().unwrap();
                        let now = update_and_send_snapshot(&mut tm);
                        if let Some(nanos) = tm.nanos_to_update(now) {
                            debug!("Updater: waiting for up to {} ns", nanos);
                            timeout = Duration::from_nanos(nanos.into());
                        } else {
                            panic!("Failed to get nanos to update");
                        }
                    }
                }
            }
        });

        let snapshot = Default::default();

        Self {
            tm,
            updater_handle,
            frame_sender,
            state_recv,
            snapshot,
        }
    }
}

impl epi::App for RefBoxApp {
    fn name(&self) -> &str {
        "UWH Ref Box"
    }

    fn setup(&mut self, _ctx: &egui::Context, frame: &Frame, _storage: Option<&dyn Storage>) {
        debug!("Setup started");
        self.frame_sender.send(frame.clone()).unwrap();
        debug!("Setup finished");
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &Frame) {
        if let Ok(snapshot) = self.state_recv.try_recv() {
            self.snapshot = snapshot;
        }

        egui::TopBottomPanel::bottom("timeout-ribbon").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.centered_and_justified(|ui| {
                    ui.button("WHITE\nTIMEOUT");
                });
                ui.button("REF\nTIMEOUT");
                ui.button("PENALTY\nSHOT");
                ui.button("BLACK\nTIMEOUT");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            ui.label(format!("Game Time: {}", self.snapshot.secs_in_period));
        });
    }
}
