//! Everything the control page shows and controls, shared between the web server, the refbox
//! connections and background jobs.

use crate::{
    BoxError,
    config::{Config, CourtConfig},
    google_auth::{self, GoogleAuth},
    live::{self, Outcome},
    portal::{self, EventPlan, video_title},
    prepare,
    refbox::{self, RefboxEvent},
    switcher::{Action, Command, CourtSwitcher, Phase, SwitchRules},
    vmix,
    youtube::YouTube,
};
use log::{info, warn};
use serde::Serialize;
use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use time::{OffsetDateTime, macros::format_description};
use tokio::{
    sync::{MappedMutexGuard, Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard, mpsc},
    task::JoinHandle,
};

pub const TOKEN_FILE: &str = "youtube-token.json";
const COURT_LOG_LINES: usize = 12;

pub struct App {
    pub config_path: PathBuf,
    pub config_dir: PathBuf,
    inner: Mutex<Inner>,
    youtube: AsyncMutex<Option<YouTube>>,
    sessions: Mutex<HashSet<String>>,
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
    youtube_units: u32,
    sign_in: String,
}

struct CourtRuntime {
    config: CourtConfig,
    switcher: CourtSwitcher,
    refbox_connected: bool,
    last_snapshot: Option<Instant>,
    log: VecDeque<String>,
    /// A switch is being carried out on YouTube/vMix right now.
    busy: bool,
    /// Why the last switch failed, until the next one succeeds.
    error: Option<String>,
    vmix_reachable: Option<bool>,
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
    pub youtube_units: u32,
    pub sign_in: String,
    /// Email of the saved portal sign-in used for watch links, if any.
    pub portal_login: Option<String>,
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
    pub vmix_address: String,
    pub vmix: &'static str,
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
            last_snapshot: None,
            log: VecDeque::new(),
            busy: false,
            error: None,
            vmix_reachable: None,
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
                youtube_units: 0,
                sign_in: String::new(),
            }),
            youtube: AsyncMutex::new(None),
            sessions: Mutex::new(HashSet::new()),
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

    pub fn state_file(&self) -> PathBuf {
        prepare::state_path(&self.config_dir, &self.inner().config.event_slug)
    }

    pub fn plan(&self) -> Option<EventPlan> {
        self.inner().plan.clone()
    }

    // ----- Sessions (PIN) -----

    pub fn add_session(&self, token: String) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(token);
    }

    pub fn has_session(&self, token: &str) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(token)
    }

    pub fn remove_session(&self, token: &str) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(token);
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
            let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
            executors.push(action_tx);
            let app = Arc::clone(self);
            let court_config = court.clone();
            tasks.push(tokio::spawn(async move {
                while let Some(action) = action_rx.recv().await {
                    app.execute(generation, i, &court_config, action).await;
                }
            }));
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
                court.note("Refbox connected".into());
            }
            RefboxEvent::Disconnected => {
                court.refbox_connected = false;
                court.note("Refbox connection lost — automatic switching paused".into());
            }
            RefboxEvent::Snapshot(snapshot) => {
                court.last_snapshot = Some(Instant::now());
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
        let result = court.switcher.on_command(command);
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

    /// Carries out one switching decision on YouTube/vMix (runs on the court's worker).
    async fn execute(
        self: &Arc<Self>,
        generation: u64,
        i: usize,
        court: &CourtConfig,
        action: Action,
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

    // ----- Settings and schedule -----

    /// Saves new settings. Court changes are refused while a court's day is running.
    pub fn apply_settings(self: &Arc<Self>, new: Config) -> Result<(), String> {
        new.validate()?;
        let courts_changed;
        {
            let mut inner = self.inner();
            courts_changed = inner.config.courts != new.courts;
            let day_running = inner.courts.iter().any(|c| c.switcher.status().day_running);
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
        tokio::spawn(async move { app.refresh_plan().await });
        Ok(())
    }

    pub async fn refresh_plan(&self) {
        let (url, slug) = {
            let inner = self.inner();
            (
                inner.config.portal_url.clone(),
                inner.config.event_slug.clone(),
            )
        };
        if slug.is_empty() {
            return;
        }
        let result = portal::fetch_event_plan(&url, &slug).await;
        let mut inner = self.inner();
        if inner.config.event_slug != slug {
            return; // settings changed meanwhile
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
            }
            Err(e) => {
                warn!("Couldn't load the schedule: {e}");
                inner.plan_error = Some(format!("Couldn't load the schedule: {e}"));
            }
        }
    }

    // ----- YouTube -----

    /// The YouTube connection, opened on first use. Held for the whole call, so only one
    /// YouTube operation runs at a time.
    pub async fn youtube(&self) -> Result<MappedMutexGuard<'_, YouTube>, BoxError> {
        let mut guard = self.youtube.lock().await;
        if guard.is_none() {
            if !self.token_file().exists() {
                return Err(
                    "Not connected to YouTube yet: open Settings and press Connect YouTube".into(),
                );
            }
            let auth = GoogleAuth::load(&self.client_file(), &self.token_file())?;
            *guard = Some(YouTube::new(auth));
        }
        AsyncMutexGuard::try_map(guard, |yt| yt.as_mut())
            .map_err(|_| "YouTube connection unavailable".into())
    }

    pub fn record_youtube(&self, channel: Option<String>, units: u32) {
        let mut inner = self.inner();
        if channel.is_some() {
            inner.youtube_channel = channel;
        }
        inner.youtube_units = units;
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
        let inner = self.inner();
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
                let stale = c.last_snapshot.is_none_or(|t| t.elapsed().as_secs() > 10);
                CourtStatus {
                    name: c.config.name.clone(),
                    refbox_address: format!("{}:{}", c.config.refbox_ip, c.config.refbox_port),
                    refbox: match (c.refbox_connected, stale) {
                        (false, _) => "disconnected",
                        (true, true) => "no data",
                        (true, false) => "ok",
                    },
                    day_running: s.day_running,
                    hold: s.hold,
                    live_title: s.live.as_deref().map(|g| describe(plan, g)),
                    live: s.live,
                    phase,
                    game_title: game.as_deref().map(|g| describe(plan, g)),
                    game,
                    in_rosters: secs_left.is_some_and(|secs| {
                        (inner.config.roster_end_secs..=inner.config.roster_start_secs)
                            .contains(&secs)
                    }),
                    secs_left,
                    secs_until_switch: s.secs_until_switch,
                    secs_until_rosters: s.secs_until_rosters,
                    log: c.log.iter().cloned().collect(),
                    busy: c.busy,
                    error: c.error.clone(),
                    vmix_address: c.config.vmix_address.clone(),
                    vmix: match c.vmix_reachable {
                        None => "checking",
                        Some(true) => "ok",
                        Some(false) => "unreachable",
                    },
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
            youtube_units: inner.youtube_units,
            sign_in: inner.sign_in.clone(),
            portal_login: crate::portal_links::saved_email(&self.config_dir),
            job: inner.job.clone(),
            courts,
        }
    }
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
