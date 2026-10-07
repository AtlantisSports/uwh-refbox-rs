//! Everything the control page shows and controls, shared between the web server, the refbox
//! connections and background jobs.

use crate::{
    BoxError,
    access::{PinFailures, Sessions},
    companion,
    config::{Config, CourtConfig},
    google_auth::{self, GoogleAuth},
    live::{self, Outcome},
    portal::{self, EventPlan, video_title},
    prepare,
    quota::{self, Ledger},
    recovery,
    refbox::{self, RefboxEvent},
    switcher::{Action, Command, CourtSwitcher, Phase, Status as SwitchStatus, SwitchRules},
    title_sync, vmix,
    youtube::YouTube,
};
use log::{info, warn};
use serde::Serialize;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use time::{OffsetDateTime, macros::format_description};
use tokio::{
    sync::{
        MappedMutexGuard, Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard,
        mpsc::{self, UnboundedReceiver},
    },
    task::JoinHandle,
};

pub const TOKEN_FILE: &str = "youtube-token.json";
const COURT_LOG_LINES: usize = 12;
/// How often a running court's upcoming titles are checked against the portal (ADR 026 §2).
const TITLE_SYNC_EVERY: Duration = Duration::from_secs(600);
/// The longest one court's 10-minute check may take before it is given up.
const TITLE_SYNC_WAIT: Duration = Duration::from_secs(60);
/// How often the Stream Deck's live status is brought up to date in Companion (ADR 026 §4).
const COMPANION_EVERY: Duration = Duration::from_secs(1);

pub struct App {
    pub config_path: PathBuf,
    pub config_dir: PathBuf,
    inner: Mutex<Inner>,
    youtube: AsyncMutex<Option<YouTube>>,
    sessions: Mutex<Sessions>,
    /// Held while a wrong PIN waits for its answer, so wrong PINs are answered one at a time.
    pin_failures: AsyncMutex<PinFailures>,
}

struct Inner {
    config: Config,
    plan: Option<EventPlan>,
    plan_error: Option<String>,
    courts: Vec<CourtRuntime>,
    /// Bumped whenever the courts are reconfigured, so messages from old refbox connections
    /// are ignored.
    generation: u64,
    refbox_tasks: Vec<JoinHandle<()>>,
    refbox_tx: Option<mpsc::UnboundedSender<(u64, usize, RefboxEvent)>>,
    /// One worker per court carries out its switches on YouTube/vMix, one at a time.
    executors: Vec<mpsc::UnboundedSender<Action>>,
    job: JobStatus,
    youtube_channel: Option<String>,
    sign_in: String,
}

struct CourtRuntime {
    config: CourtConfig,
    switcher: CourtSwitcher,
    refbox_connected: bool,
    /// A game update has been read on the current connection. The refbox only sends while
    /// something changes, so a quiet connection is still a good one.
    refbox_has_data: bool,
    /// Why the refbox's data couldn't be read, until a readable update arrives.
    refbox_unreadable: Option<String>,
    /// The break countdown was running, so the refbox should be sending every second, but
    /// nothing has come for [`refbox::SILENCE`]. Until the next snapshot, the refbox counts as
    /// not responding; automatic switching waits for it, as when it is disconnected.
    refbox_silent: bool,
    log: VecDeque<String>,
    /// A switch is being carried out on YouTube/vMix right now.
    busy: bool,
    /// Why the last switch failed, until the next one succeeds.
    error: Option<String>,
    vmix_reachable: Option<bool>,
    /// Games reported as gone from the portal since Start day or End day, in the order found.
    /// Each is noted in the log once, and stays listed on the court's card.
    removed_reported: Vec<String>,
    /// Why the Stream Deck's live status couldn't be sent to Companion, until it next succeeds.
    companion_error: Option<String>,
    /// Restart recovery has asked YouTube about this court's videos, or its day was started or
    /// ended by hand, so recovery doesn't look at it again.
    recovered: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct JobStatus {
    pub name: String,
    pub running: bool,
    pub log: Vec<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Status {
    pub practice_mode: bool,
    pub event_slug: String,
    pub event_name: Option<String>,
    pub schedule_error: Option<String>,
    pub portal_url: String,
    pub privacy: String,
    pub youtube_connected: bool,
    pub youtube_channel: Option<String>,
    /// What's left of this program's share of today's YouTube allowance, and the share itself.
    pub quota_remaining: u32,
    pub quota_share: u32,
    /// The allowance is low: the chat message and "Next game" link are being skipped.
    pub extras_paused: bool,
    pub sign_in: String,
    pub job: JobStatus,
    pub courts: Vec<CourtStatus>,
}

#[derive(Serialize)]
pub struct CourtStatus {
    pub name: String,
    pub refbox_address: String,
    pub refbox: &'static str,
    pub day_running: bool,
    pub hold: bool,
    pub live: Option<String>,
    pub live_title: Option<String>,
    pub phase: &'static str,
    pub game: Option<String>,
    pub game_title: Option<String>,
    pub secs_left: Option<u32>,
    pub secs_until_switch: Option<u32>,
    pub secs_until_rosters: Option<u32>,
    pub in_rosters: bool,
    pub log: Vec<String>,
    pub busy: bool,
    pub error: Option<String>,
    /// Games with a video that are no longer on the portal (the videos were kept).
    pub removed_games: Vec<String>,
    pub vmix_address: String,
    pub vmix: &'static str,
    /// Why the Stream Deck's live status couldn't be sent to Companion, until it next succeeds.
    pub companion_error: Option<String>,
}

fn clock() -> String {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(format_description!("[hour]:[minute]:[second]"))
        .unwrap_or_default()
}

impl CourtRuntime {
    fn new(config: CourtConfig, rules: SwitchRules) -> Self {
        Self {
            config,
            switcher: CourtSwitcher::new(rules),
            refbox_connected: false,
            refbox_has_data: false,
            refbox_unreadable: None,
            refbox_silent: false,
            log: VecDeque::new(),
            busy: false,
            error: None,
            vmix_reachable: None,
            removed_reported: Vec::new(),
            companion_error: None,
            recovered: false,
        }
    }

    fn note(&mut self, line: String) {
        info!("[Court {}] {line}", self.config.name);
        self.log.push_front(format!("{} {line}", clock()));
        self.log.truncate(COURT_LOG_LINES);
    }
}

fn rules_of(config: &Config) -> SwitchRules {
    SwitchRules {
        switch_lead_secs: config.switch_lead_secs,
        roster_start_secs: config.roster_start_secs,
        roster_end_secs: config.roster_end_secs,
    }
}

fn describe(plan: Option<&EventPlan>, game: &str) -> String {
    plan.and_then(|p| p.game(game).map(|g| video_title(&p.event_name, g)))
        .unwrap_or_else(|| format!("Game {game}"))
}

fn describe_action(plan: Option<&EventPlan>, action: &Action, practice: bool) -> String {
    let prefix = if practice { "(practice) " } else { "" };
    match action {
        Action::GoLive(game) => format!("{prefix}GO LIVE: {}", describe(plan, game)),
        Action::Switch { from, to } => {
            format!("{prefix}SWITCH: Game {from} → {}", describe(plan, to))
        }
        Action::End(game) => format!("{prefix}END DAY: Game {game} ended"),
    }
}

impl App {
    pub fn new(config_path: PathBuf, config: Config) -> Arc<Self> {
        let config_dir = config_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let rules = rules_of(&config);
        let courts = config
            .courts
            .iter()
            .map(|c| CourtRuntime::new(c.clone(), rules))
            .collect();
        Arc::new(Self {
            config_path,
            config_dir,
            inner: Mutex::new(Inner {
                config,
                plan: None,
                plan_error: None,
                courts,
                generation: 0,
                refbox_tasks: Vec::new(),
                refbox_tx: None,
                executors: Vec::new(),
                job: JobStatus::default(),
                youtube_channel: None,
                sign_in: String::new(),
            }),
            youtube: AsyncMutex::new(None),
            sessions: Mutex::new(Sessions::default()),
            pin_failures: AsyncMutex::new(PinFailures::default()),
        })
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves the data usable; keep going.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn config(&self) -> Config {
        self.inner().config.clone()
    }

    pub fn token_file(&self) -> PathBuf {
        self.config_dir.join(TOKEN_FILE)
    }

    pub fn client_file(&self) -> PathBuf {
        self.config_dir
            .join(&self.inner().config.client_secret_file)
    }

    pub fn ledger_file(&self) -> PathBuf {
        self.config_dir.join(quota::LEDGER_FILE)
    }

    pub fn state_file(&self) -> Result<PathBuf, BoxError> {
        prepare::state_path(&self.config_dir, &self.inner().config.event_slug)
    }

    pub fn plan(&self) -> Option<EventPlan> {
        self.inner().plan.clone()
    }

    /// Replaces the cached schedule with one just fetched for `event_slug`, as
    /// [`App::refresh_plan`] does. Ignored if the event has changed meanwhile.
    pub fn set_plan(&self, event_slug: &str, plan: EventPlan) {
        let mut inner = self.inner();
        if inner.config.event_slug == event_slug {
            inner.plan = Some(plan);
            inner.plan_error = None;
        }
    }

    /// The game whose video the court's switcher has live, if any.
    pub fn live_game(&self, court_name: &str) -> Option<String> {
        self.inner()
            .courts
            .iter()
            .find(|c| c.config.name == court_name)
            .and_then(|c| c.switcher.status().live)
    }

    /// Of `removed`, the games not yet reported for this court since Start day or End day.
    /// They count as reported from now on.
    pub fn newly_removed(&self, court_name: &str, removed: &[String]) -> Vec<String> {
        let mut inner = self.inner();
        let Some(court) = inner
            .courts
            .iter_mut()
            .find(|c| c.config.name == court_name)
        else {
            return Vec::new();
        };
        let new: Vec<String> = removed
            .iter()
            .filter(|game| !court.removed_reported.contains(game))
            .cloned()
            .collect();
        court.removed_reported.extend(new.iter().cloned());
        new
    }

    // ----- Sessions (PIN) -----

    pub fn add_session(&self, token: String) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add(token, Instant::now());
    }

    pub fn has_session(&self, token: &str) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .has(token, Instant::now())
    }

    pub fn remove_session(&self, token: &str) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(token);
    }

    /// Waits before a wrong PIN (from `from`) is answered: longer for each one in a row (see
    /// [`PinFailures::record`]). Wrong PINs wait one after another, never side by side, so
    /// guesses can't be sped up by sending many at once. A correct PIN never waits here.
    pub async fn wrong_pin(&self, from: &str) {
        let mut failures = self.pin_failures.lock().await;
        let wait = failures.record(Instant::now());
        warn!("Wrong PIN from {from}; answering in {} s", wait.as_secs());
        tokio::time::sleep(wait).await;
    }

    // ----- Refbox connections and switching -----

    /// (Re)connects to every configured court's refbox.
    pub fn start_refbox_connections(self: &Arc<Self>) {
        let (tx, mut rx) = mpsc::unbounded_channel::<(u64, usize, RefboxEvent)>();
        let mut inner = self.inner();
        for task in inner.refbox_tasks.drain(..) {
            task.abort();
        }
        inner.generation += 1;
        let generation = inner.generation;
        let mut tasks = Vec::new();
        for (i, court) in inner.config.courts.iter().enumerate() {
            let (court_tx, mut court_rx) = mpsc::unbounded_channel();
            tasks.push(tokio::spawn(refbox::follow_refbox(
                i,
                court.refbox_ip,
                court.refbox_port,
                court_tx,
            )));
            let tx = tx.clone();
            tasks.push(tokio::spawn(async move {
                while let Some((i, event)) = court_rx.recv().await {
                    if tx.send((generation, i, event)).is_err() {
                        return;
                    }
                }
            }));
        }
        let mut executors = Vec::new();
        for (i, court) in inner.config.courts.iter().enumerate() {
            let (action_tx, action_rx) = mpsc::unbounded_channel::<Action>();
            executors.push(action_tx);
            let app = Arc::clone(self);
            tasks.push(tokio::spawn(app.run_worker(
                generation,
                i,
                court.clone(),
                action_rx,
            )));
            let app = Arc::clone(self);
            let address = court.vmix_address.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    let ok = vmix::is_reachable(&address).await;
                    app.set_vmix_reachable(generation, i, ok);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }));
        }
        inner.refbox_tasks = tasks;
        inner.refbox_tx = Some(tx);
        inner.executors = executors;
        drop(inner);

        let app = Arc::clone(self);
        tokio::spawn(async move {
            while let Some((generation, i, event)) = rx.recv().await {
                app.on_refbox_event(generation, i, event);
            }
        });
    }

    fn on_refbox_event(&self, generation: u64, i: usize, event: RefboxEvent) {
        let mut inner = self.inner();
        if generation != inner.generation {
            return;
        }
        let practice = inner.config.practice_mode;
        let Inner {
            courts,
            plan,
            executors,
            ..
        } = &mut *inner;
        let Some(court) = courts.get_mut(i) else {
            return;
        };
        match event {
            RefboxEvent::Connected => {
                court.refbox_connected = true;
                court.refbox_has_data = false;
                court.refbox_silent = false;
                court.note("Refbox connected".into());
            }
            RefboxEvent::Disconnected => {
                court.refbox_connected = false;
                court.refbox_has_data = false;
                court.refbox_silent = false;
                court.note("Refbox connection lost — automatic switching paused".into());
            }
            RefboxEvent::Unreadable(reason) => {
                court.refbox_unreadable = Some(reason.clone());
                court.note(format!(
                    "✖ The refbox sends data Stream Manager can't read. Check the refbox port is \
                     8000 (8001 is the LED panel's). Details: {reason}"
                ));
            }
            // Only a silence during the break counts: the refbox also goes quiet while a game's
            // clock is stopped, which can last minutes. The break countdown always runs.
            RefboxEvent::Silent => {
                let in_break = matches!(court.switcher.status().phase, Phase::Break { .. });
                if court.refbox_has_data && in_break && !court.refbox_silent {
                    court.refbox_silent = true;
                    court.note(
                        "✖ The refbox has stopped sending its countdown (not responding) — \
                         automatic switching paused"
                            .into(),
                    );
                }
            }
            RefboxEvent::Snapshot(snapshot) => {
                court.refbox_has_data = true;
                court.refbox_unreadable = None;
                if court.refbox_silent {
                    court.refbox_silent = false;
                    court.note("Refbox responding again".into());
                }
                if let Some(action) = court.switcher.on_snapshot(&snapshot) {
                    dispatch(court, executors.get(i), plan.as_ref(), action, practice);
                }
            }
        }
    }

    /// A button on the Live tab or Companion. Returns a short message for the operator.
    pub fn court_command(&self, court_name: &str, command: Command) -> Result<String, String> {
        let mut inner = self.inner();
        let practice = inner.config.practice_mode;
        let Inner {
            courts,
            plan,
            executors,
            ..
        } = &mut *inner;
        let i = courts
            .iter()
            .position(|c| c.config.name == court_name)
            .ok_or_else(|| format!("There is no court \"{court_name}\""))?;
        let court = &mut courts[i];
        if court.busy
            && matches!(
                command,
                Command::SwitchNow | Command::StartDay | Command::EndDay
            )
        {
            return Err("Please wait: the previous switch is still being carried out".to_string());
        }
        let was_hold = court.switcher.status().hold;
        let was_running = court.switcher.status().day_running;
        let result = court.switcher.on_command(command);
        // Start day or End day took effect: the removed games are looked at afresh, and restart
        // recovery leaves the court alone from now on.
        if was_running != court.switcher.status().day_running {
            court.removed_reported.clear();
            court.recovered = true;
        }
        if let Some(action) = result {
            let message = describe_action(plan.as_ref(), &action, practice);
            dispatch(court, executors.get(i), plan.as_ref(), action, practice);
            return Ok(message);
        }
        let message = match command {
            Command::StartDay => {
                if court.switcher.status().day_running {
                    "The day is already running".to_string()
                } else {
                    "Can't start yet: no data from this court's refbox".to_string()
                }
            }
            Command::Hold => "HOLD ON — automatic switching paused".to_string(),
            Command::Release if was_hold => {
                "Hold released — will switch at the next safe moment".to_string()
            }
            Command::Release => "Hold was not on".to_string(),
            Command::SwitchNow => {
                "Nothing to switch to (day not started, or no next game)".to_string()
            }
            Command::EndDay => "The day isn't running".to_string(),
        };
        court.note(message.clone());
        Ok(message)
    }

    /// A court's worker: carries out its switching decisions one at a time.
    async fn run_worker(
        self: Arc<Self>,
        generation: u64,
        i: usize,
        court: CourtConfig,
        mut actions: UnboundedReceiver<Action>,
    ) {
        while let Some(action) = actions.recv().await {
            self.execute(generation, i, &court, action, &mut actions)
                .await;
        }
    }

    /// Carries out one switching decision on YouTube/vMix (runs on the court's worker). If it
    /// fails, the decisions queued behind it are dropped: they were made before the failure and
    /// would start from a game that isn't live.
    async fn execute(
        self: &Arc<Self>,
        generation: u64,
        i: usize,
        court: &CourtConfig,
        action: Action,
        queued: &mut UnboundedReceiver<Action>,
    ) {
        self.with_court(generation, i, |c| c.busy = true);
        let plan = self.plan();
        let log_app = Arc::clone(self);
        let mut log = move |line: String| log_app.with_court(generation, i, |c| c.note(line));
        let outcome = live::carry_out(self, court, plan.as_ref(), &action, &mut log).await;
        self.with_court(generation, i, |c| {
            c.busy = false;
            match outcome {
                Outcome::Done => c.error = None,
                Outcome::Warnings(warnings) => {
                    c.error = None;
                    for w in warnings {
                        c.note(format!("⚠ {w}"));
                    }
                }
                Outcome::Failed {
                    actually_live,
                    error,
                } => {
                    c.switcher.switch_failed(actually_live);
                    let advice = if c.switcher.status().day_running {
                        "Hold is ON. Fix the problem, then press Switch now."
                    } else {
                        "Fix the problem, then press Start day."
                    };
                    c.note(format!("✖ {error} — {advice}"));
                    c.error = Some(format!("{error} — {advice}"));
                    // Emptied in the same step as the failure is recorded, so nothing the
                    // operator sends after it is lost.
                    let mut dropped = Vec::new();
                    while let Ok(next) = queued.try_recv() {
                        dropped.push(describe_action(plan.as_ref(), &next, false));
                    }
                    if !dropped.is_empty() {
                        c.note(format!(
                            "✖ Not carried out, because the switch before it failed: {}",
                            dropped.join("; ")
                        ));
                    }
                }
            }
        });
    }

    fn with_court(&self, generation: u64, i: usize, f: impl FnOnce(&mut CourtRuntime)) {
        let mut inner = self.inner();
        if inner.generation == generation {
            if let Some(court) = inner.courts.get_mut(i) {
                f(court);
            }
        }
    }

    fn set_vmix_reachable(&self, generation: u64, i: usize, ok: bool) {
        self.with_court(generation, i, |c| {
            if c.vmix_reachable != Some(ok) {
                c.vmix_reachable = Some(ok);
                if !ok {
                    c.note(format!("vMix not reachable at {}", c.config.vmix_address));
                }
            }
        });
    }

    // ----- Restart recovery -----

    /// After a restart, or when the courts change, asks YouTube which of each court's videos
    /// today is live and carries on from it (ADR 026 §9). Without a schedule, prepared videos or
    /// a YouTube connection nothing changes, and the court waits for Start day.
    ///
    /// A court is looked at until YouTube has been asked about it once, or until its day is
    /// started or ended by hand. So if the schedule or YouTube wasn't available the first time,
    /// recovery runs again after the next schedule load (see [`App::refresh_plan_and_recover`]).
    /// After resuming, vMix's output for the live video is started again (it is off after a
    /// reboot), except in practice mode.
    pub async fn recover_live_videos(self: &Arc<Self>) {
        let (generation, courts, plan, slug, practice) = {
            let inner = self.inner();
            (
                inner.generation,
                courts_to_recover(&inner),
                inner.plan.clone(),
                inner.config.event_slug.clone(),
                inner.config.practice_mode,
            )
        };
        if courts.is_empty() {
            return;
        }
        let Some(plan) = plan.filter(|_| !slug.is_empty()) else {
            return;
        };
        let state = match prepare::state_path(&self.config_dir, &slug)
            .and_then(|path| prepare::load_state(&path, &slug))
        {
            Ok(state) => state,
            Err(e) => {
                warn!("Couldn't read the prepared videos to look for a live one: {e}");
                return;
            }
        };
        let now = OffsetDateTime::now_utc();
        for (i, court) in courts {
            let videos = recovery::todays_videos(&plan, &state, &court.name, now);
            if videos.is_empty() {
                // No video today, so none can be live.
                self.with_court(generation, i, |c| c.recovered = true);
                continue;
            }
            // Not connected to YouTube: nothing to resume from (yet).
            let Ok(mut youtube) = self.youtube().await else {
                return;
            };
            let found = recovery::find_live(&mut youtube, &videos).await;
            drop(youtube);
            match found {
                Ok((Some(live), also_live)) => {
                    let mut resumed = false;
                    self.with_court(generation, i, |c| {
                        c.recovered = true;
                        resumed = resume_court(c, &live, &also_live);
                    });
                    if resumed && !practice {
                        self.restart_output(generation, i, &court, &state, &live)
                            .await;
                    }
                }
                Ok((None, _)) => self.with_court(generation, i, |c| c.recovered = true),
                Err(e) => warn!(
                    "[Court {}] Couldn't check which video is live: {e}",
                    court.name
                ),
            }
        }
    }

    /// Starts vMix's output for a resumed live video again: after a reboot vMix's outputs are
    /// off. Starting one that is already running changes nothing.
    async fn restart_output(
        &self,
        generation: u64,
        i: usize,
        court: &CourtConfig,
        state: &prepare::EventState,
        live: &str,
    ) {
        let line = match live::resume_destination(court, state, live) {
            Ok(destination) => {
                match vmix::start_destination(&court.vmix_address, destination).await {
                    Ok(()) => format!("vMix: started destination {destination}"),
                    Err(e) => format!(
                        "⚠ Couldn't start vMix destination {destination} for Game {live}: {e}. \
                         Start it in vMix."
                    ),
                }
            }
            Err(e) => format!("⚠ {e}: start Game {live}'s vMix destination by hand"),
        };
        self.with_court(generation, i, |c| c.note(line));
    }

    // ----- Portal title sync -----

    /// Every 10 minutes, brings each running court's upcoming videos in line with the portal
    /// (ADR 026 §2). A court is skipped while its share of the allowance is low; the check just
    /// before each switch still runs. Runs for as long as the program does.
    pub async fn run_title_sync(self: &Arc<Self>) {
        let mut every = tokio::time::interval(TITLE_SYNC_EVERY);
        // A slow round doesn't bring the next checks forward.
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // The first tick is immediate: the first check comes 10 minutes after start.
        every.tick().await;
        loop {
            every.tick().await;
            let (generation, courts) = {
                let inner = self.inner();
                (inner.generation, courts_to_sync(&inner))
            };
            for (i, court) in courts {
                if !self.extras_allowed(&court.name) {
                    continue;
                }
                let log_app = Arc::clone(self);
                let mut log =
                    move |line: String| log_app.with_court(generation, i, |c| c.note(line));
                // Giving up drops the check, which releases the YouTube connection for switches.
                let check = title_sync::sync_court(self, &court, None, &mut log);
                let problem = match tokio::time::timeout(TITLE_SYNC_WAIT, check).await {
                    Ok(Ok(_)) => None,
                    Ok(Err(e)) => Some(format!(
                        "⚠ Couldn't check the titles against the portal: {e}"
                    )),
                    Err(_) => Some(
                        "⚠ The portal or YouTube didn't answer in time; titles weren't checked"
                            .to_string(),
                    ),
                };
                if let Some(line) = problem {
                    self.with_court(generation, i, |c| c.note(line));
                }
            }
        }
    }

    /// Every second, sends each court's Hold, rosters, now live and up next to Companion's
    /// custom variables, but only the values that changed (ADR 026 §4). Off while the Companion
    /// address is empty. Runs on its own, so a slow or missing Companion never holds up a switch.
    pub async fn run_companion_sync(self: &Arc<Self>) {
        let mut every = tokio::time::interval(COMPANION_EVERY);
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last = companion::LastSent::default();
        loop {
            every.tick().await;
            // Read everything first: the lock is never held while waiting for Companion.
            let (address, generation, courts) = {
                let inner = self.inner();
                (
                    companion::normalise_address(&inner.config.companion_address),
                    inner.generation,
                    companion_values(&inner),
                )
            };
            if address.is_empty() {
                last.forget();
                let mut inner = self.inner();
                for court in &mut inner.courts {
                    court.companion_error = None;
                }
                continue;
            }
            // Values Companion lost (e.g. it restarted) come back within a minute.
            last.resend_all_if_due(std::time::Instant::now());
            for (i, wanted) in courts {
                // A court with a failed value has all its values sent again next second.
                let error = companion::send_court(&mut last, &address, &wanted).await;
                self.with_court(generation, i, |c| c.companion_error = error);
            }
        }
    }

    // ----- Settings and schedule -----

    /// Saves new settings. Court changes are refused while a court's day is running.
    pub fn apply_settings(self: &Arc<Self>, new: Config) -> Result<(), String> {
        new.validate_for_save()?;
        let courts_changed;
        {
            let mut inner = self.inner();
            courts_changed = inner.config.courts != new.courts;
            let day_running = inner.courts.iter().any(|c| c.switcher.status().day_running);
            for running in inner
                .courts
                .iter()
                .filter(|c| c.switcher.status().day_running)
            {
                let mode_changed = new.courts.iter().any(|c| {
                    c.name == running.config.name && c.stream_mode != running.config.stream_mode
                });
                if mode_changed {
                    return Err(format!(
                        "End the day on Court {} before changing its stream keys setting",
                        running.config.name
                    ));
                }
            }
            let risky = courts_changed
                || rules_of(&inner.config) != rules_of(&new)
                || inner.config.event_slug != new.event_slug
                || inner.config.portal_url != new.portal_url
                || inner.config.practice_mode != new.practice_mode;
            if risky && day_running {
                return Err(
                    "End the day on every court before changing courts, event, timing or practice mode"
                        .to_string(),
                );
            }
            confy::store_path(&self.config_path, &new)
                .map_err(|e| format!("Couldn't save settings: {e}"))?;
            let event_changed = inner.config.event_slug != new.event_slug
                || inner.config.portal_url != new.portal_url;
            let rules_changed = rules_of(&inner.config) != rules_of(&new);
            if event_changed {
                inner.plan = None;
                inner.plan_error = None;
            }
            if courts_changed || rules_changed {
                let rules = rules_of(&new);
                inner.courts = new
                    .courts
                    .iter()
                    .map(|c| CourtRuntime::new(c.clone(), rules))
                    .collect();
            }
            inner.config = new;
        }
        if courts_changed {
            self.start_refbox_connections();
        }
        let app = Arc::clone(self);
        tokio::spawn(async move { app.refresh_plan_and_recover().await });
        Ok(())
    }

    /// Loads the schedule and, if that worked, runs restart recovery for the courts it hasn't
    /// covered yet (e.g. because the first schedule load failed).
    pub async fn refresh_plan_and_recover(self: &Arc<Self>) {
        if self.refresh_plan().await {
            self.recover_live_videos().await;
        }
    }

    /// Loads the schedule from the portal. Returns whether that worked.
    pub async fn refresh_plan(&self) -> bool {
        let (url, slug) = {
            let inner = self.inner();
            (
                inner.config.portal_url.clone(),
                inner.config.event_slug.clone(),
            )
        };
        if slug.is_empty() {
            return false;
        }
        let result = portal::fetch_event_plan(&url, &slug).await;
        let mut inner = self.inner();
        if inner.config.event_slug != slug {
            return false; // settings changed meanwhile
        }
        match result {
            Ok(plan) => {
                info!(
                    "Loaded schedule for {} ({} games)",
                    plan.event_name,
                    plan.games.len()
                );
                inner.plan = Some(plan);
                inner.plan_error = None;
                true
            }
            Err(e) => {
                warn!("Couldn't load the schedule: {e}");
                inner.plan_error = Some(format!("Couldn't load the schedule: {e}"));
                false
            }
        }
    }

    // ----- YouTube -----

    /// The YouTube connection, opened on first use. Only one holder at a time, so only one
    /// YouTube operation runs at a time. A switch holds it for its whole run; longer jobs
    /// (Prepare, the 10-minute title check) take it one game at a time through
    /// [`YouTubeAccess`](crate::youtube::YouTubeAccess), so a switch never waits long.
    pub async fn youtube(&self) -> Result<MappedMutexGuard<'_, YouTube>, BoxError> {
        let mut guard = self.youtube.lock().await;
        if guard.is_none() {
            if !self.token_file().exists() {
                return Err(
                    "Not connected to YouTube yet: open Settings and press Connect YouTube".into(),
                );
            }
            let auth = GoogleAuth::load(&self.client_file(), &self.token_file())?;
            *guard = Some(YouTube::new(auth, Some(self.ledger_file()))?);
        }
        AsyncMutexGuard::try_map(guard, |yt| yt.as_mut())
            .map_err(|_| "YouTube connection unavailable".into())
    }

    /// Remembers the connected channel's name for the page.
    pub fn record_youtube(&self, channel: Option<String>) {
        if channel.is_some() {
            self.inner().youtube_channel = channel;
        }
    }

    /// Units this program has used today, from the allowance ledger.
    fn quota_used_today(&self) -> u32 {
        Ledger::load(&self.ledger_file()).used_today(OffsetDateTime::now_utc())
    }

    /// Whether the extras (chat message, "Next game" link) may still run for this court: what's
    /// left of the share must still cover the rest of today's switches plus a margin (ADR 026 §7).
    pub fn extras_allowed(&self, court_name: &str) -> bool {
        let used = self.quota_used_today();
        let inner = self.inner();
        extras_allowed_with(&inner, court_name, used, OffsetDateTime::now_utc())
    }

    pub async fn forget_youtube(&self) {
        *self.youtube.lock().await = None;
        self.inner().youtube_channel = None;
    }

    pub fn set_sign_in(&self, message: &str) {
        self.inner().sign_in = message.to_string();
    }

    // ----- Background jobs (prepare, clean-up) -----

    /// Marks a job as started; fails if another is still running.
    pub fn begin_job(&self, name: &str) -> Result<(), String> {
        let mut inner = self.inner();
        if inner.job.running {
            return Err(format!(
                "Please wait: \"{}\" is still running",
                inner.job.name
            ));
        }
        inner.job = JobStatus {
            name: name.to_string(),
            running: true,
            log: Vec::new(),
            error: None,
        };
        Ok(())
    }

    pub fn job_log(&self, line: String) {
        info!("{line}");
        self.inner().job.log.push(line);
    }

    pub fn end_job(&self, error: Option<String>) {
        let mut inner = self.inner();
        inner.job.running = false;
        inner.job.error = error;
    }

    // ----- Status for the page -----

    pub fn status(&self) -> Status {
        let connected = google_auth::is_connected(&self.token_file());
        let quota_used = self.quota_used_today();
        let now = OffsetDateTime::now_utc();
        let inner = self.inner();
        let quota_share = quota::share(
            inner.config.quota_daily_limit,
            inner.config.quota_share_percent,
        );
        let extras_paused = inner
            .courts
            .iter()
            .any(|c| !extras_allowed_with(&inner, &c.config.name, quota_used, now));
        let plan = inner.plan.as_ref();
        let courts = inner
            .courts
            .iter()
            .map(|c| {
                let s = c.switcher.status();
                let (phase, game, secs_left) = match &s.phase {
                    Phase::Unknown => ("unknown", None, None),
                    Phase::Playing(game) => ("playing", Some(game.clone()), None),
                    Phase::Break {
                        upcoming,
                        secs_left,
                    } => ("break", Some(upcoming.clone()), Some(*secs_left)),
                };

                CourtStatus {
                    name: c.config.name.clone(),
                    refbox_address: format!("{}:{}", c.config.refbox_ip, c.config.refbox_port),
                    refbox: match (
                        c.refbox_connected,
                        c.refbox_has_data,
                        c.refbox_unreadable.is_some(),
                    ) {
                        (false, _, _) => "disconnected",
                        (true, true, _) if c.refbox_silent => "not_responding",
                        (true, true, _) => "ok",
                        (true, false, true) => "unreadable",
                        (true, false, false) => "waiting",
                    },
                    day_running: s.day_running,
                    hold: s.hold,
                    live_title: s.live.as_deref().map(|g| describe(plan, g)),
                    live: s.live,
                    phase,
                    game_title: game.as_deref().map(|g| describe(plan, g)),
                    game,
                    in_rosters: in_rosters(&inner.config, secs_left),
                    secs_left,
                    secs_until_switch: s.secs_until_switch,
                    secs_until_rosters: s.secs_until_rosters,
                    log: c.log.iter().cloned().collect(),
                    busy: c.busy,
                    error: c.error.clone(),
                    removed_games: c.removed_reported.clone(),
                    vmix_address: c.config.vmix_address.clone(),
                    vmix: match c.vmix_reachable {
                        None => "checking",
                        Some(true) => "ok",
                        Some(false) => "unreachable",
                    },
                    companion_error: c.companion_error.clone(),
                }
            })
            .collect();
        Status {
            practice_mode: inner.config.practice_mode,
            event_slug: inner.config.event_slug.clone(),
            event_name: plan.map(|p| p.event_name.clone()),
            schedule_error: inner.plan_error.clone(),
            portal_url: inner.config.portal_url.clone(),
            privacy: inner.config.privacy.clone(),
            youtube_connected: connected,
            youtube_channel: inner.youtube_channel.clone(),
            quota_remaining: quota_share.saturating_sub(quota_used),
            quota_share,
            extras_paused,
            sign_in: inner.sign_in.clone(),
            job: inner.job.clone(),
            courts,
        }
    }
}

/// Switches still to come today on `court_name` and, as the share covers the whole program, on
/// every other court whose day is running.
fn switches_left(inner: &Inner, court_name: &str, now: OffsetDateTime) -> u32 {
    let Some(plan) = inner.plan.as_ref() else {
        return 0;
    };
    inner
        .courts
        .iter()
        .filter(|c| c.config.name == court_name || c.switcher.status().day_running)
        .map(|c| {
            let status = c.switcher.status();
            let next = match &status.phase {
                Phase::Unknown => None,
                Phase::Playing(game) => Some(game.as_str()),
                Phase::Break { upcoming, .. } => Some(upcoming.as_str()),
            };
            quota::court_switches_left(plan, &c.config.name, status.live.as_deref(), next, now)
        })
        .fold(0, u32::saturating_add)
}

fn extras_allowed_with(inner: &Inner, court_name: &str, used: u32, now: OffsetDateTime) -> bool {
    let share = quota::share(
        inner.config.quota_daily_limit,
        inner.config.quota_share_percent,
    );
    quota::extras_allowed(
        share.saturating_sub(used),
        switches_left(inner, court_name, now),
    )
}

/// The overlay is showing rosters: the break countdown is inside the roster window.
fn in_rosters(config: &Config, secs_left: Option<u32>) -> bool {
    secs_left
        .is_some_and(|secs| (config.roster_end_secs..=config.roster_start_secs).contains(&secs))
}

/// What one court's Stream Deck buttons show, from its switcher's status (ADR 026 §4).
fn companion_state<'a>(status: &'a SwitchStatus, config: &Config) -> companion::CourtState<'a> {
    let secs_left = match &status.phase {
        Phase::Break { secs_left, .. } => Some(*secs_left),
        Phase::Unknown | Phase::Playing(_) => None,
    };
    companion::CourtState {
        hold: status.hold,
        secs_until_rosters: status.secs_until_rosters,
        in_rosters: in_rosters(config, secs_left),
        live: status.live.as_deref().filter(|_| status.day_running),
        next: status.next.as_deref(),
    }
}

/// Each court's Companion variables and their values, by court index.
fn companion_values(inner: &Inner) -> Vec<(usize, Vec<(String, String)>)> {
    inner
        .courts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let status = c.switcher.status();
            let state = companion_state(&status, &inner.config);
            (i, companion::court_pairs(&c.config.name, &state))
        })
        .collect()
}

/// Courts the 10-minute title check covers: those whose day is running, unless in practice
/// mode, where nothing is sent to YouTube.
fn courts_to_sync(inner: &Inner) -> Vec<(usize, CourtConfig)> {
    if inner.config.practice_mode {
        return Vec::new();
    }
    inner
        .courts
        .iter()
        .enumerate()
        .filter(|(_, c)| c.switcher.status().day_running)
        .map(|(i, c)| (i, c.config.clone()))
        .collect()
}

/// The courts restart recovery still has to look at: not yet covered, and the day not running.
fn courts_to_recover(inner: &Inner) -> Vec<(usize, CourtConfig)> {
    inner
        .courts
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.recovered && !c.switcher.status().day_running)
        .map(|(i, c)| (i, c.config.clone()))
        .collect()
}

/// Carries on from `live` after a restart, unless the court's day is already running. Returns
/// whether it did.
fn resume_court(court: &mut CourtRuntime, live: &str, also_live: &[String]) -> bool {
    if court.switcher.status().day_running {
        return false;
    }
    court.switcher.resume(live.to_string());
    court.note(format!("Resumed: Game {live} is live"));
    for other in also_live {
        court.note(format!(
            "⚠ Game {other} is also still live on YouTube; end it from YouTube Studio"
        ));
    }
    true
}

/// Logs a switching decision and, unless in practice mode, hands it to the court's worker.
fn dispatch(
    court: &mut CourtRuntime,
    executor: Option<&mpsc::UnboundedSender<Action>>,
    plan: Option<&EventPlan>,
    action: Action,
    practice: bool,
) {
    court.note(describe_action(plan, &action, practice));
    if practice {
        return;
    }
    court.error = None;
    match executor {
        Some(tx) if tx.send(action).is_ok() => court.busy = true,
        _ => court.note("✖ Internal error: no switching worker for this court".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::parse_event_plan;
    use time::macros::datetime;
    use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};

    fn snapshot(period: GamePeriod, game: &str, next: &str, secs: u32) -> GameSnapshot {
        GameSnapshot {
            current_period: period,
            secs_in_period: secs,
            game_number: game.to_string(),
            next_game_number: next.to_string(),
            ..Default::default()
        }
    }

    /// The four Stream Deck values after feeding `snapshots`, starting the day after the first
    /// one when `start` is set.
    fn buttons(start: bool, hold: bool, snapshots: &[GameSnapshot]) -> [String; 4] {
        let config = Config::default();
        let mut switcher = CourtSwitcher::new(rules_of(&config));
        for (i, snap) in snapshots.iter().enumerate() {
            switcher.on_snapshot(snap);
            if i == 0 && start {
                switcher.on_command(Command::StartDay);
            }
            if i == 0 && hold {
                switcher.on_command(Command::Hold);
            }
        }
        let status = switcher.status();
        companion::court_values(&companion_state(&status, &config))
    }

    fn values(hold: &str, rosters: &str, now: &str, next: &str) -> [String; 4] {
        [hold, rosters, now, next].map(String::from)
    }

    #[test]
    fn stream_deck_shows_the_next_game_while_playing() {
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        assert_eq!(
            buttons(true, false, &[playing]),
            values("OFF", "", "Now: Game 14", "Next: Game 15")
        );
    }

    #[test]
    fn stream_deck_counts_down_to_the_rosters_before_the_switch() {
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        let early_break = snapshot(GamePeriod::BetweenGames, "14", "15", 240);
        assert_eq!(
            buttons(true, false, &[playing, early_break]),
            values("OFF", "Rosters in 0:59", "Now: Game 14", "Next: Game 15")
        );
    }

    #[test]
    fn stream_deck_shows_rosters_on_screen() {
        // Held, so the switch hasn't happened when the rosters come up.
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        let rosters = snapshot(GamePeriod::BetweenGames, "14", "15", 100);
        assert_eq!(
            buttons(true, true, &[playing, rosters]),
            values("ON", "Rosters on screen", "Now: Game 14", "Next: Game 15")
        );
    }

    #[test]
    fn stream_deck_after_the_switch_has_no_next_game_until_the_refbox_names_one() {
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        let switched = snapshot(GamePeriod::BetweenGames, "14", "15", 190);
        assert_eq!(
            buttons(true, false, &[playing, switched]),
            values("OFF", "Rosters in 0:09", "Now: Game 15", "")
        );
        // Once 15 kicks off, the refbox names 16 as its next game.
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        let switched = snapshot(GamePeriod::BetweenGames, "14", "15", 190);
        let kickoff = snapshot(GamePeriod::FirstHalf, "15", "16", 600);
        assert_eq!(
            buttons(true, false, &[playing, switched, kickoff]),
            values("OFF", "", "Now: Game 15", "Next: Game 16")
        );
    }

    #[test]
    fn stream_deck_before_start_day_shows_the_game_start_day_would_put_live() {
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        assert_eq!(
            buttons(false, false, &[playing]),
            values("OFF", "", "", "Next: Game 14")
        );
        let between = snapshot(GamePeriod::BetweenGames, "0", "1", 240);
        assert_eq!(
            buttons(false, false, &[between]),
            values("OFF", "Rosters in 0:59", "", "Next: Game 1")
        );
        assert_eq!(buttons(false, false, &[]), values("OFF", "", "", ""));
    }

    #[test]
    fn stream_deck_shows_hold_on_and_off() {
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        assert_eq!(buttons(true, true, std::slice::from_ref(&playing))[0], "ON");
        assert_eq!(buttons(true, false, &[playing])[0], "OFF");
    }

    fn temp_app(name: &str) -> Arc<App> {
        let dir =
            std::env::temp_dir().join(format!("stream-manager-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        App::new(dir.join("config.toml"), Config::default())
    }

    #[test]
    fn resume_court_carries_on_from_the_live_video_and_reports_others() {
        let rules = rules_of(&Config::default());
        let court_config = Config::default().courts.remove(0);
        let mut court = CourtRuntime::new(court_config.clone(), rules);
        assert!(resume_court(&mut court, "14", &["13".to_string()]));
        let status = court.switcher.status();
        assert!(status.day_running);
        assert_eq!(status.live.as_deref(), Some("14"));
        let log: Vec<&String> = court.log.iter().collect();
        assert_eq!(log.len(), 2);
        assert!(log[1].ends_with(" Resumed: Game 14 is live"), "{log:?}");
        assert!(
            log[0]
                .ends_with(" ⚠ Game 13 is also still live on YouTube; end it from YouTube Studio"),
            "{log:?}"
        );

        // The operator pressed Start day while recovery was asking YouTube: leave it alone.
        let mut court = CourtRuntime::new(court_config, rules);
        court.switcher.resume("7".into());
        assert!(!resume_court(&mut court, "14", &[]));
        assert_eq!(court.switcher.status().live.as_deref(), Some("7"));
        assert!(court.log.is_empty());
    }

    #[test]
    fn a_refbox_silent_during_the_break_shows_as_not_responding_until_it_sends_again() {
        let app = temp_app("silent");
        let generation = app.inner().generation;
        let refbox = |app: &App| app.status().courts[0].refbox;
        let send = |event| app.on_refbox_event(generation, 0, event);
        send(RefboxEvent::Connected);
        // Quiet before anything arrived: still just waiting for the first update.
        send(RefboxEvent::Silent);
        assert_eq!(refbox(&app), "waiting");
        // Quiet during a game (its clock may be stopped): not a fault.
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        send(RefboxEvent::Snapshot(Box::new(playing)));
        send(RefboxEvent::Silent);
        assert_eq!(refbox(&app), "ok");
        // Quiet during the break, whose countdown always runs: not responding.
        let between = snapshot(GamePeriod::BetweenGames, "14", "15", 240);
        send(RefboxEvent::Snapshot(Box::new(between.clone())));
        send(RefboxEvent::Silent);
        assert_eq!(refbox(&app), "not_responding");
        assert!(
            app.inner().courts[0].log[0].contains("not responding"),
            "{:?}",
            app.inner().courts[0].log
        );
        send(RefboxEvent::Snapshot(Box::new(between)));
        assert_eq!(refbox(&app), "ok");
        assert!(app.inner().courts[0].log[0].ends_with("Refbox responding again"));
        // A reconnect starts afresh.
        send(RefboxEvent::Silent);
        send(RefboxEvent::Disconnected);
        assert_eq!(refbox(&app), "disconnected");
        send(RefboxEvent::Connected);
        assert_eq!(refbox(&app), "waiting");
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[tokio::test]
    async fn a_failed_switch_drops_the_actions_queued_behind_it() {
        let app = temp_app("dropped");
        app.inner().config.event_slug = "test-cup".to_string();
        let court = app.config().courts.remove(0);
        let (tx, rx) = mpsc::unbounded_channel();
        // Not connected to YouTube, so the first action fails.
        tx.send(Action::GoLive("1".into())).unwrap();
        tx.send(Action::Switch {
            from: "1".into(),
            to: "2".into(),
        })
        .unwrap();
        drop(tx);
        let generation = app.inner().generation;
        Arc::clone(&app).run_worker(generation, 0, court, rx).await;

        let log: Vec<String> = app.inner().courts[0].log.iter().cloned().collect();
        // Newest first: the dropped switch, then the failure; the switch was never tried.
        assert_eq!(log.len(), 2, "{log:?}");
        assert!(
            log[0].ends_with(
                " ✖ Not carried out, because the switch before it failed: SWITCH: Game 1 → Game 2"
            ),
            "{log:?}"
        );
        assert!(log[1].contains("✖ Not connected to YouTube yet"), "{log:?}");
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[tokio::test]
    async fn recovery_waits_for_a_schedule_and_stops_once_the_day_is_started() {
        let app = temp_app("recover");
        app.inner().config.event_slug = "test-cup".to_string();
        let pending = |app: &App| -> Vec<usize> {
            courts_to_recover(&app.inner())
                .into_iter()
                .map(|(i, _)| i)
                .collect()
        };
        assert_eq!(pending(&app), [0]);
        // The first schedule load failed: recovery can't run yet, so the court stays pending.
        app.recover_live_videos().await;
        assert_eq!(pending(&app), [0]);

        // With a schedule but no video for today, nothing can be live: the court is covered.
        app.inner().plan =
            Some(parse_event_plan(r#"{ "event": { "name": "Test Cup" }, "games": [] }"#).unwrap());
        app.recover_live_videos().await;
        assert!(pending(&app).is_empty());
        let _ = std::fs::remove_dir_all(&app.config_dir);

        // A court whose day is started by hand is never resumed afterwards.
        let app = temp_app("recover-started");
        app.inner().courts[0].switcher.resume("3".into());
        assert!(pending(&app).is_empty());
        let _ = app.court_command("1", Command::EndDay);
        assert!(pending(&app).is_empty());
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn a_removed_game_is_reported_once_and_listed_until_the_day_starts_or_ends() {
        let app = temp_app("removed");
        app.inner().courts[0].switcher.resume("20".into());
        let removed = ["22".to_string()];
        assert_eq!(app.newly_removed("1", &removed), ["22"]);
        assert!(app.newly_removed("1", &removed).is_empty());
        assert_eq!(
            app.newly_removed("1", &["22".to_string(), "23".to_string()]),
            ["23"]
        );
        // The card keeps listing them after the log line.
        assert_eq!(app.status().courts[0].removed_games, ["22", "23"]);

        // End day refused while a switch is still running: the list stays.
        app.inner().courts[0].busy = true;
        assert!(app.court_command("1", Command::EndDay).is_err());
        assert_eq!(app.status().courts[0].removed_games, ["22", "23"]);

        // End day taking effect starts afresh.
        app.inner().courts[0].busy = false;
        assert!(app.court_command("1", Command::EndDay).is_ok());
        assert!(app.status().courts[0].removed_games.is_empty());
        assert_eq!(app.newly_removed("1", &removed), ["22"]);

        // End day with the day not running changes nothing, so the list stays.
        let _ = app.court_command("1", Command::EndDay);
        assert_eq!(app.status().courts[0].removed_games, ["22"]);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn the_title_check_covers_running_courts_outside_practice_mode() {
        let app = temp_app("sync-courts");
        app.inner().courts[0].switcher.resume("20".into());
        // Practice mode is on by default: nothing is sent to YouTube.
        assert!(courts_to_sync(&app.inner()).is_empty());
        app.inner().config.practice_mode = false;
        let courts: Vec<String> = courts_to_sync(&app.inner())
            .into_iter()
            .map(|(_, c)| c.name)
            .collect();
        assert_eq!(courts, ["1"]);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn status_shows_whats_left_of_the_share_from_the_ledger() {
        let app = temp_app("status");
        let status = app.status();
        assert_eq!((status.quota_remaining, status.quota_share), (5_000, 5_000));
        assert!(!status.extras_paused);

        quota::record_to_file(&app.ledger_file(), 4_850, OffsetDateTime::now_utc()).unwrap();
        let status = app.status();
        assert_eq!((status.quota_remaining, status.quota_share), (150, 5_000));
        // 150 left doesn't cover even the 200-unit margin.
        assert!(status.extras_paused);
        assert!(!app.extras_allowed("1"));
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn extras_need_todays_remaining_switches_plus_margin() {
        let app = temp_app("extras");
        let game = |number: &str, start: &str| {
            format!(
                r#"{{ "number": "{number}", "startsOn": "{start}", "court": "1",
                    "dark": {{ "assignment": null }}, "light": {{ "assignment": null }} }}"#
            )
        };
        let plan = parse_event_plan(&format!(
            r#"{{ "event": {{ "name": "Test Cup" }}, "games": [ {}, {}, {} ] }}"#,
            game("1", "2026-08-01T09:00:00+10:00"),
            game("2", "2026-08-01T10:00:00+10:00"),
            game("3", "2026-08-01T11:00:00+10:00"),
        ))
        .unwrap();
        let now = datetime!(2026-08-01 08:00 +10);
        let inner = app.inner();
        // No schedule: only the margin is needed (share 5,000).
        assert!(extras_allowed_with(&inner, "1", 4_800, now));
        assert!(!extras_allowed_with(&inner, "1", 4_801, now));
        drop(inner);

        app.inner().plan = Some(plan);
        let inner = app.inner();
        // Three games today: 3 × 120 + 200 = 560 needed.
        assert!(extras_allowed_with(&inner, "1", 4_440, now));
        assert!(!extras_allowed_with(&inner, "1", 4_441, now));
        drop(inner);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }
}
