//! Everything the control page shows and controls, shared between the web server, the refbox
//! connections and background jobs.

use crate::{
    BoxError,
    access::{self, Devices, Sessions, SignIn, SignInLine},
    companion,
    config::{Config, CourtConfig},
    google_auth::{self, GoogleAuth},
    live::{self, Outcome},
    portal::{self, EventPlan, video_title},
    prepare,
    quota::{self, LedgerFile},
    recovery,
    refbox::{self, RefboxEvent},
    switcher::{Action, Command, CourtSwitcher, Phase, Status as SwitchStatus, SwitchRules},
    title_sync, vmix,
    youtube::YouTube,
};
use log::{info, warn};
use serde::Serialize;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use time::{OffsetDateTime, macros::format_description};
use tokio::{
    sync::{
        Mutex as AsyncMutex, OwnedMutexGuard,
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
    /// The YouTube connection, opened on first use. Each user gets its own handle on it.
    youtube: Mutex<Option<YouTube>>,
    /// One lock per court name, held by everything that works on that court's videos (see
    /// [`App::court_lock`]).
    court_locks: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
    sessions: Mutex<Sessions>,
    /// Every PIN sign-in waits here for its turn.
    sign_ins: SignInLine,
    /// The devices allowed to reach the control page, as they were when Stream Manager started.
    pub devices: Devices,
    /// When each court's vMix destinations were last stopped by a switch or End day.
    stopped_keys: Mutex<live::StoppedKeys>,
    /// This program's use of today's YouTube allowance, read from its file once at start and
    /// kept up to date as each YouTube call is counted.
    ledger: Arc<LedgerFile>,
    /// Whether the YouTube sign-in file exists, as last checked: at start, and whenever the
    /// YouTube connection is dropped (e.g. after connecting).
    youtube_connected: AtomicBool,
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
        let devices = Devices::from_config(&config);
        let courts = config
            .courts
            .iter()
            .map(|c| CourtRuntime::new(c.clone(), rules))
            .collect();
        Arc::new(Self {
            config_path,
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
            youtube: Mutex::new(None),
            court_locks: Mutex::new(HashMap::new()),
            sessions: Mutex::new(Sessions::default()),
            sign_ins: SignInLine::default(),
            devices,
            stopped_keys: Mutex::new(live::StoppedKeys::default()),
            ledger: Arc::new(LedgerFile::open(config_dir.join(quota::LEDGER_FILE))),
            youtube_connected: AtomicBool::new(google_auth::is_connected(
                &config_dir.join(TOKEN_FILE),
            )),
            config_dir,
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

    pub fn state_file(&self) -> Result<PathBuf, BoxError> {
        prepare::state_path(&self.config_dir, &self.inner().config.event_slug)
    }

    pub fn plan(&self) -> Option<EventPlan> {
        self.inner().plan.clone()
    }

    /// Replaces the cached schedule with one just fetched for `event_slug` from `portal_url`, as
    /// [`App::refresh_plan`] does. Ignored if the event or the portal has changed meanwhile.
    pub fn set_plan(&self, portal_url: &str, event_slug: &str, plan: EventPlan) {
        let mut inner = self.inner();
        if fetched_from_current(&inner.config, portal_url, event_slug) {
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

    /// Whether the court's day is running right now.
    pub fn day_running(&self, court_name: &str) -> bool {
        self.inner()
            .courts
            .iter()
            .any(|c| c.config.name == court_name && c.switcher.status().day_running)
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

    /// Signs out every session, e.g. after the PIN was changed.
    pub fn clear_sessions(&self) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Checks a PIN typed at the sign-in (from `from`), in the one line every sign-in from
    /// another device waits in (see [`SignInLine`]). The PIN is compared only once the attempt
    /// reaches the front. A sign-in on the mini PC itself (`on_this_pc`) is checked at once.
    pub async fn sign_in(&self, pin: &str, from: &str, on_this_pc: bool) -> SignIn {
        let outcome = self
            .sign_ins
            .attempt(on_this_pc, || {
                access::secret_matches(pin.trim(), &self.config().pin)
            })
            .await;
        match outcome {
            SignIn::Right => {}
            SignIn::Wrong => warn!("Wrong PIN from {from}"),
            SignIn::TooMany => warn!("Sign-in from {from} turned away: too many waiting"),
        }
        outcome
    }

    /// Whether the device settings saved now differ from those in use since Stream Manager
    /// started.
    pub fn devices_need_restart(&self) -> bool {
        Devices::from_config(&self.config()) != self.devices
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
                let in_break = matches!(
                    court.switcher.status().phase,
                    Phase::Break { .. } | Phase::Finished
                );
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
                let status = court.switcher.status();
                if status.day_running {
                    "The day is already running".to_string()
                } else if status.phase == Phase::Finished {
                    "No more games on this court".to_string()
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
    /// recovery runs again after the next schedule load (see [`App::refresh_plan_and_recover`])
    /// and after YouTube is connected.
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
            let Ok(mut youtube) = self.youtube() else {
                return;
            };
            // Asked while no switch, End day or Go live is under way on this court.
            let court_lock = self.court_lock(&court.name).await;
            let found = recovery::find_live(&mut youtube, &videos).await;
            drop(court_lock);
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
                // Giving up drops the check, which releases the court's lock for its switches.
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

    /// Saves `new` in place of the settings in use (tests only; the web page edits through
    /// [`App::update_settings`]).
    #[cfg(test)]
    pub fn apply_settings(self: &Arc<Self>, new: Config) -> Result<(), String> {
        self.update_settings(|_| Ok(new))
    }

    /// Saves new settings, made by `edit` from the settings in use under the same lock as the
    /// save, so two changes made at the same moment (e.g. a new button key and a settings save
    /// from another device) can't undo each other. Court changes are refused while a court's day
    /// is running. Changes to the courts, the event or the portal are refused while a court's
    /// worker is still carrying out a switch or End day.
    pub fn update_settings<E: From<String>>(
        self: &Arc<Self>,
        edit: impl FnOnce(&Config) -> Result<Config, E>,
    ) -> Result<(), E> {
        let courts_changed;
        {
            let mut inner = self.inner();
            let new = edit(&inner.config)?;
            new.validate_for_save(&inner.config)?;
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
                    )
                    .into());
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
                        .to_string()
                        .into(),
                );
            }
            // Changing the courts restarts every court, and changing the event or the portal
            // drops the schedule, so a worker still carrying out End day or a switch would lose
            // track of it.
            let event_or_portal_changed = inner.config.event_slug != new.event_slug
                || inner.config.portal_url != new.portal_url;
            let busy = inner
                .courts
                .iter()
                .find(|c| c.busy)
                .filter(|_| courts_changed || event_or_portal_changed);
            if let Some(busy) = busy {
                return Err(format!(
                    "Court {} is still finishing its last action; try again in a moment.",
                    busy.config.name
                )
                .into());
            }
            save_config(&self.config_path, &new)
                .map_err(|e| format!("Couldn't save settings: {e}"))?;
            let event_changed = event_or_portal_changed;
            let rules_changed = rules_of(&inner.config) != rules_of(&new);
            if event_changed {
                inner.plan = None;
                inner.plan_error = None;
            }
            let rules = rules_of(&new);
            if courts_changed {
                inner.courts = new
                    .courts
                    .iter()
                    .map(|c| CourtRuntime::new(c.clone(), rules))
                    .collect();
            } else if rules_changed {
                // Same courts and the same refbox connections: only the timing changes, so each
                // court keeps its connection state, last game update and log.
                for court in &mut inner.courts {
                    court.switcher.set_rules(rules);
                }
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
        if !fetched_from_current(&inner.config, &url, &slug) {
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

    /// A handle on the YouTube connection, opened on first use. Handles share the sign-in and
    /// the allowance count, so YouTube calls for different courts run at the same time. Keeping
    /// one court's work apart is [`App::court_lock`]'s job.
    pub fn youtube(&self) -> Result<YouTube, BoxError> {
        let mut connection = self.youtube.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(youtube) = connection.as_ref() {
            return Ok(youtube.handle());
        }
        if !self.token_file().exists() {
            return Err(
                "Not connected to YouTube yet: open Settings and press Connect YouTube".into(),
            );
        }
        let auth = GoogleAuth::load(&self.client_file(), &self.token_file())?;
        let youtube = YouTube::new(auth, Some(Arc::clone(&self.ledger)))?;
        let handle = youtube.handle();
        *connection = Some(youtube);
        Ok(handle)
    }

    /// The lock for one court's videos. A switch (with its title check), End day, Go live,
    /// restart recovery, and each per-game step of Prepare and of the 10-minute title check
    /// hold it, so no two of them work on the same court at once, while different courts'
    /// switches run side by side.
    ///
    /// No deadlock: a holder never waits for another court's lock (only
    /// [`App::lock_all_courts`] takes several, always in name order), and the other locks taken
    /// while holding it (the sign-in, the record file, the allowance ledger, the shared state)
    /// are only ever held briefly and never wait for a court.
    pub async fn court_lock(&self, court_name: &str) -> OwnedMutexGuard<()> {
        let lock = Arc::clone(
            self.court_locks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(court_name.to_string())
                .or_default(),
        );
        lock.lock_owned().await
    }

    /// Every configured court's lock, taken in name order, for a job that works on every
    /// court's videos at once (deleting the test videos).
    pub async fn lock_all_courts(&self) -> Vec<OwnedMutexGuard<()>> {
        let mut names: Vec<String> = self
            .config()
            .courts
            .iter()
            .map(|c| c.name.clone())
            .collect();
        names.sort();
        names.dedup();
        let mut guards = Vec::with_capacity(names.len());
        for name in &names {
            guards.push(self.court_lock(name).await);
        }
        guards
    }

    /// Remembers the connected channel's name for the page.
    pub fn record_youtube(&self, channel: Option<String>) {
        if channel.is_some() {
            self.inner().youtube_channel = channel;
        }
    }

    /// Units this program has used today, from the allowance ledger (kept in memory).
    fn quota_used_today(&self) -> u32 {
        self.ledger.used_today(OffsetDateTime::now_utc())
    }

    /// Whether the extras (chat message, "Next game" link) may still run for this court: what's
    /// left of the share must still cover the rest of today's switches plus a margin (ADR 026 §7).
    pub fn extras_allowed(&self, court_name: &str) -> bool {
        let used = self.quota_used_today();
        let inner = self.inner();
        extras_allowed_with(&inner, court_name, used, OffsetDateTime::now_utc())
    }

    /// Notes that a switch or End day stopped `court_name`'s vMix `destination` just now.
    pub fn note_stopped(&self, court_name: &str, destination: u8) {
        self.stopped_keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .record(court_name, destination, Instant::now());
    }

    /// Whether a switch or End day stopped `court_name`'s vMix `destination` in the last minute.
    pub fn stopped_recently(&self, court_name: &str, destination: u8) -> bool {
        self.stopped_keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .recently(court_name, destination, Instant::now())
    }

    /// Drops the YouTube connection (it is opened again on next use), e.g. after connecting
    /// afresh, and checks again whether the sign-in file exists.
    pub fn forget_youtube(&self) {
        *self.youtube.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.inner().youtube_channel = None;
        self.youtube_connected.store(
            google_auth::is_connected(&self.token_file()),
            Ordering::SeqCst,
        );
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
        let connected = self.youtube_connected.load(Ordering::SeqCst);
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
                    Phase::Finished => ("finished", None, None),
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
                Phase::Unknown | Phase::Finished => None,
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
        Phase::Unknown | Phase::Playing(_) | Phase::Finished => None,
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

/// Whether a schedule fetched for `event_slug` from `portal_url` still belongs to the settings.
fn fetched_from_current(config: &Config, portal_url: &str, event_slug: &str) -> bool {
    config.event_slug == event_slug && config.portal_url == portal_url
}

/// Saves the settings in the format confy reads (TOML), replacing the file whole: a crash
/// part-way leaves the old file, never an empty one (which would load as the defaults, without
/// the PIN).
fn save_config(path: &Path, config: &Config) -> Result<(), BoxError> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} isn't a file name", path.display()))?
        .to_string_lossy();
    // confy writes its TOML to a scratch file, which is then read back and written over the
    // settings file in one step.
    let scratch = path.with_file_name(format!(".{name}.unsaved"));
    let text = confy::store_path(&scratch, config)
        .map_err(BoxError::from)
        .and_then(|()| std::fs::read_to_string(&scratch).map_err(BoxError::from));
    let _ = std::fs::remove_file(&scratch);
    prepare::write_atomically(path, &text?)
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
    fn stream_deck_after_the_last_game_has_no_next_game_and_no_rosters() {
        // The refbox sends a blank upcoming game after the court's last game.
        let last = snapshot(GamePeriod::SecondHalf, "20", "", 300);
        let finished = snapshot(GamePeriod::BetweenGames, "20", "", 100);
        assert_eq!(
            buttons(true, false, &[last, finished.clone()]),
            values("OFF", "", "Now: Game 20", "")
        );
        let finished_early = snapshot(GamePeriod::BetweenGames, "20", "", 600);
        assert_eq!(
            buttons(false, false, &[finished_early]),
            values("OFF", "", "", "")
        );
        assert_eq!(
            buttons(false, false, &[finished]),
            values("OFF", "", "", "")
        );
    }

    #[test]
    fn stream_deck_shows_hold_on_and_off() {
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        assert_eq!(buttons(true, true, std::slice::from_ref(&playing))[0], "ON");
        assert_eq!(buttons(true, false, &[playing])[0], "OFF");
    }

    #[test]
    fn start_day_says_why_it_did_nothing() {
        let app = temp_app("start-day-message");
        assert_eq!(
            app.court_command("1", Command::StartDay),
            Ok("Can't start yet: no data from this court's refbox".to_string())
        );
        // After the court's last game the refbox sends a blank upcoming game.
        app.inner().courts[0].switcher.on_snapshot(&snapshot(
            GamePeriod::BetweenGames,
            "20",
            "",
            600,
        ));
        assert_eq!(
            app.court_command("1", Command::StartDay),
            Ok("No more games on this court".to_string())
        );
        let _ = std::fs::remove_dir_all(&app.config_dir);
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

    #[tokio::test]
    async fn changing_only_the_timing_keeps_each_courts_refbox_connection() {
        let app = temp_app("timing");
        let generation = app.inner().generation;
        app.on_refbox_event(generation, 0, RefboxEvent::Connected);
        let playing = snapshot(GamePeriod::SecondHalf, "14", "15", 300);
        app.on_refbox_event(generation, 0, RefboxEvent::Snapshot(Box::new(playing)));
        assert_eq!(app.status().courts[0].refbox, "ok");

        let mut new = app.config();
        new.switch_lead_secs += 10;
        app.apply_settings(new).unwrap();
        assert_eq!(app.status().courts[0].refbox, "ok");
        assert_eq!(app.inner().generation, generation);
        // The last game update is kept too.
        let switcher = app.inner().courts[0].switcher.status();
        assert_eq!(switcher.next.as_deref(), Some("14"));
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[tokio::test]
    async fn court_changes_wait_for_a_court_still_carrying_out_a_switch() {
        let app = temp_app("busy-settings");
        app.inner().courts[0].busy = true;
        let mut new = app.config();
        new.courts[0].refbox_port += 1;
        assert_eq!(
            app.apply_settings(new.clone()),
            Err("Court 1 is still finishing its last action; try again in a moment.".to_string())
        );
        assert_ne!(app.config().courts, new.courts);

        // Nor the event or the portal (e.g. while End day is still being carried out).
        let mut event = app.config();
        event.event_slug = "another-cup".into();
        let mut portal = app.config();
        portal.portal_url = crate::config::LIVE_PORTAL_URL.into();
        for changed in [event, portal] {
            assert_eq!(
                app.apply_settings(changed),
                Err(
                    "Court 1 is still finishing its last action; try again in a moment."
                        .to_string()
                )
            );
        }
        assert_eq!(app.config().event_slug, "");

        // Other settings can still be saved.
        let mut timing = app.config();
        timing.switch_lead_secs += 10;
        assert!(app.apply_settings(timing).is_ok());
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn day_running_answers_for_the_court_at_the_moment_it_is_asked() {
        let app = temp_app("day-running");
        assert!(!app.day_running("1"));
        app.inner().courts[0].switcher.resume("3".into());
        assert!(app.day_running("1"));
        assert!(!app.day_running("2"));
        assert!(app.court_command("1", Command::EndDay).is_ok());
        assert!(!app.day_running("1"));
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

        app.ledger.record(4_850, OffsetDateTime::now_utc()).unwrap();
        let status = app.status();
        assert_eq!((status.quota_remaining, status.quota_share), (150, 5_000));
        // 150 left doesn't cover even the 200-unit margin.
        assert!(status.extras_paused);
        assert!(!app.extras_allowed("1"));
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn status_reads_the_allowance_from_memory_not_the_file() {
        let app = temp_app("status-memory");
        app.ledger.record(1_000, OffsetDateTime::now_utc()).unwrap();
        // An unreadable file isn't looked at by the status.
        std::fs::write(app.config_dir.join(quota::LEDGER_FILE), "not json").unwrap();
        assert_eq!(app.status().quota_remaining, 4_000);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[tokio::test]
    async fn different_courts_can_be_worked_on_at_once_but_one_court_cannot() {
        let app = temp_app("court-locks");
        let wait = |ms| Duration::from_millis(ms);
        let court_1 = app.court_lock("1").await;
        // Another court's switch doesn't wait for court 1's.
        let court_2 = tokio::time::timeout(wait(1_000), app.court_lock("2")).await;
        assert!(court_2.is_ok(), "court 2 is free while court 1 is busy");
        // Court 1's other work does.
        let again = tokio::time::timeout(wait(100), app.court_lock("1")).await;
        assert!(again.is_err(), "court 1 can't be worked on twice at once");
        // Taking every court waits for the busy one.
        let all = tokio::time::timeout(wait(100), app.lock_all_courts()).await;
        assert!(all.is_err());
        drop(court_1);
        let again = tokio::time::timeout(wait(1_000), app.court_lock("1")).await;
        assert!(again.is_ok(), "free again once court 1's work is done");
        drop(again);
        let all = tokio::time::timeout(wait(1_000), app.lock_all_courts()).await;
        assert_eq!(all.map(|guards| guards.len()).ok(), Some(1));
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn a_schedule_fetched_before_the_portal_changed_is_thrown_away() {
        let app = temp_app("stale-portal");
        app.inner().config.event_slug = "test-cup".to_string();
        let plan =
            || parse_event_plan(r#"{ "event": { "name": "Test Cup" }, "games": [] }"#).unwrap();
        app.set_plan("https://old-portal.example", "test-cup", plan());
        assert!(app.plan().is_none(), "fetched from the portal used before");
        app.set_plan(&app.config().portal_url, "other-cup", plan());
        assert!(app.plan().is_none(), "fetched for the event chosen before");
        app.set_plan(&app.config().portal_url, "test-cup", plan());
        assert!(app.plan().is_some());
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[test]
    fn saved_settings_load_back_the_same_with_no_scratch_file_left() {
        let app = temp_app("save-settings");
        let config = Config {
            event_slug: "test-cup".to_string(),
            pin: "4321".to_string(),
            ..Config::default()
        };
        // Replaces whatever was there, even a damaged file.
        std::fs::write(&app.config_path, "half a sett").unwrap();
        save_config(&app.config_path, &config).unwrap();
        let loaded: Config = confy::load_path(&app.config_path).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&config).unwrap()
        );
        let files: Vec<_> = std::fs::read_dir(&app.config_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(files, ["config.toml"]);
        let _ = std::fs::remove_dir_all(&app.config_dir);
    }

    #[tokio::test]
    async fn the_youtube_sign_in_file_is_checked_on_connect_not_on_every_status() {
        let app = temp_app("token-cache");
        assert!(!app.status().youtube_connected);
        std::fs::write(app.token_file(), "{}").unwrap();
        assert!(!app.status().youtube_connected, "not read on every status");
        // Connecting drops the old connection, which checks the file again.
        app.forget_youtube();
        assert!(app.status().youtube_connected);
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
