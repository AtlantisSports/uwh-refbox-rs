use clap::Parser;
use coarsetime::Instant;
use crossbeam_channel::bounded;
use log::{LevelFilter, info, warn};
#[cfg(debug_assertions)]
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::{
    append::rolling_file::{
        RollingFileAppender,
        policy::compound::{
            CompoundPolicy, roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
        },
    },
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use macroquad::prelude::*;
use network::{StateUpdate, TeamInfoRaw};
use std::{cmp::Ordering, str::FromStr};
use std::{net::IpAddr, path::PathBuf};
use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};

mod flag;
mod load_images;
mod network;
use network::{BLACK_TEAM_NAME, WHITE_TEAM_NAME};
mod pages;

use load_images::Texture;

const APP_NAME: &str = "overlay";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AppConfig {
    refbox_ip: IpAddr,
    refbox_port: u16,
    uwhportal_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refbox_ip: IpAddr::from_str("127.0.0.1").unwrap(),
            refbox_port: 8000,
            uwhportal_url: String::from("https://api.uwhportal.com"),
        }
    }
}

pub struct State {
    snapshot: GameSnapshot,
    black: TeamInfo,
    white: TeamInfo,
    referees: Vec<Member>,
    pool: String,
    start_time: String,
    half_play_duration: Option<u32>,
    event_logo: Option<Texture>,
    sponsor_logo: Option<Texture>,
}

// TODO: Change this to return Result. We're not rn cause from_file_with_format
// panics anyways if image bytes is invalid
pub fn texture_from_image(image: network::Image) -> Option<Texture> {
    match texture_from_image_result(image) {
        Ok(texture) => Some(texture),
        Err(e) => {
            warn!("Failed to create texture from image: {e}");
            None
        }
    }
}

fn texture_from_image_result(image: network::Image) -> Result<Texture, Box<dyn std::error::Error>> {
    let alpha_image = alphagen::make_white_alpha_raw_rgba8(image.0, image.1, image.2.clone())?;
    Ok(Texture {
        color: Texture2D::from_rgba8(image.0, image.1, &image.2),
        alpha: Texture2D::from_rgba8(image.0, image.1, &alpha_image),
    })
}

impl State {
    fn update_state(&mut self, recieved_update: StateUpdate) {
        match recieved_update {
            StateUpdate::Snapshot(snapshot) => {
                if self.snapshot.event_id != snapshot.event_id {
                    info!("Snapshot for new event received: {:?}", snapshot.event_id);
                    self.event_logo = None;
                    self.sponsor_logo = None;

                    if self.snapshot.game_number() != snapshot.game_number() {
                        info!("Snapshot for new game received: {}", snapshot.game_number());
                        self.black = TeamInfo::with_name(BLACK_TEAM_NAME);
                        self.white = TeamInfo::with_name(WHITE_TEAM_NAME);
                        self.referees.clear();
                        self.pool = String::new();
                        self.start_time = String::new();
                        self.half_play_duration = None;
                    }
                }
                self.snapshot = snapshot;
            }
            StateUpdate::GameData(game_data) => {
                if let Some(ref event_id) = self.snapshot.event_id {
                    if game_data.event_id == *event_id
                        && game_data.game_number == *self.snapshot.game_number()
                    {
                        self.black = TeamInfo::from(game_data.black);
                        self.white = TeamInfo::from(game_data.white);
                        self.start_time = game_data.start_time;
                        self.referees = game_data.referees.into_iter().map(Member::from).collect();
                        self.pool = game_data.pool;
                    } else {
                        warn!(
                            "Received game data for incorrect game: {} in event {}",
                            game_data.game_number, game_data.event_id
                        );
                    }
                }
            }
            StateUpdate::EventLogos(event_id, logos) => {
                if self.snapshot.event_id.as_ref() == Some(&event_id) {
                    self.event_logo = logos.event_logo.and_then(texture_from_image);
                    self.sponsor_logo = logos.sponsors.and_then(texture_from_image);
                    info!(
                        "Updated event logos: Event: {}, Sponsor: {}",
                        self.event_logo.is_some(),
                        self.sponsor_logo.is_some()
                    );
                } else {
                    warn!("Received event logos for incorrect event: {}", event_id);
                }
            }
        }
    }
}

/// processed, non serialisable version of `network::MemberRaw`
#[derive(Clone)]
pub struct Member {
    name: String,
    role: Option<String>,
    number: Option<u8>,
    picture: Option<Texture>,
    geared_picture: Option<Texture>,
}

impl From<network::MemberRaw> for Member {
    fn from(member_raw: network::MemberRaw) -> Self {
        Self {
            name: member_raw.name,
            role: member_raw.role,
            number: member_raw.number,
            picture: member_raw.picture.and_then(texture_from_image),
            geared_picture: member_raw.geared_picture.and_then(texture_from_image),
        }
    }
}

/// processed, non serialisable version of `network::TeamInfoRaw`
pub struct TeamInfo {
    pub team_name: String,
    pub members: Vec<Member>,
    pub flag: Option<Texture>,
}

impl From<TeamInfoRaw> for TeamInfo {
    fn from(mut team_info_raw: TeamInfoRaw) -> Self {
        // Players get sorted by number, support members by name.
        team_info_raw
            .members
            .sort_unstable_by(|a, b| match (a.number, b.number) {
                (Some(num_a), Some(num_b)) => num_a.cmp(&num_b),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => a.name.cmp(&b.name),
            });
        Self {
            team_name: team_info_raw.team_name,
            members: team_info_raw
                .members
                .into_iter()
                .map(Member::from)
                .collect(),
            flag: team_info_raw.flag.and_then(texture_from_image),
        }
    }
}

impl TeamInfo {
    fn with_name(name: &str) -> Self {
        Self {
            team_name: name.to_string(),
            members: Vec::new(),
            flag: None,
        }
    }

    /// `number` can always be unwrapped on elements returned from here
    fn get_players(&self) -> impl Iterator<Item = &Member> {
        self.members.iter().filter(|m| m.number.is_some())
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, short, action(clap::ArgAction::Count))]
    /// Increase the log verbosity
    verbose: u8,

    #[clap(long)]
    /// Directory within which log files will be placed, default is platform dependent
    log_location: Option<PathBuf>,

    #[clap(long, default_value = "5000000")]
    /// Max size in bytes that a log file is allowed to reach before being rolled over
    log_max_file_size: u64,

    #[clap(long, default_value = "3")]
    /// Number of archived logs to keep
    num_old_logs: u32,
}

#[macroquad::main(window_conf())]
async fn main() {
    init_logging();

    let config: AppConfig = match confy::load(APP_NAME, None) {
        Ok(config) => config,
        Err(e) => {
            warn!("Failed to read config file, overwriting with default. Error: {e}");
            let config = AppConfig::default();
            confy::store(APP_NAME, None, &config).unwrap();
            config
        }
    };

    let (tx, rx) = bounded::<StateUpdate>(3);

    let net_worker = std::thread::spawn(|| {
        network::networking_thread(tx, config);
    });

    let assets = load_images::Textures::default();

    let mut local_state: State = State {
        snapshot: GameSnapshot {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 600,
            ..Default::default()
        },

        black: TeamInfo::with_name(BLACK_TEAM_NAME),
        referees: Vec::new(),
        white: TeamInfo::with_name(WHITE_TEAM_NAME),
        pool: String::new(),
        start_time: String::new(),
        half_play_duration: None,
        event_logo: None,
        sponsor_logo: None,
    };

    let mut renderer = pages::PageRenderer {
        animation_register0: Instant::now(),
        animation_register1: Instant::now(),
        animation_register2: Instant::now(),
        animation_register3: false,
        animation_register4: Instant::now(),
        assets,
        last_snapshot_timeout: None,
    };

    let mut flag_renderer = flag::Renderer::new();
    macroquad::window::miniquad::window::show_mouse(false);

    loop {
        assert!(!net_worker.is_finished(), "Networking thread panikd!");
        clear_background(BLACK);

        if let Ok(recieved_state) = rx.try_recv() {
            local_state.update_state(recieved_state);
            // sync local penalty list
            flag_renderer.synchronize_flags(&local_state);
        }

        match local_state.snapshot.current_period {
            GamePeriod::BetweenGames => {
                flag_renderer.reset();
                if let Some(duration) = local_state.snapshot.next_period_len_secs {
                    local_state.half_play_duration = Some(duration);
                }
                match local_state.snapshot.secs_in_period {
                    182..=u32::MAX => {
                        // If an old game just finished, display its scores
                        if local_state.snapshot.is_old_game {
                            renderer.final_scores(&local_state);
                        } else {
                            renderer.next_game(&local_state);
                        }
                    }
                    30..=181 => {
                        if local_state.snapshot.is_old_game {
                            renderer.final_scores(&local_state);
                        } else {
                            renderer.roster(&local_state);
                        }
                    }
                    _ => {
                        if local_state.snapshot.is_old_game
                            && local_state.snapshot.secs_in_period > 5
                        {
                            renderer.final_scores(&local_state);
                        } else {
                            renderer.pre_game_display(&local_state);
                        }
                    }
                }
            }
            GamePeriod::FirstHalf | GamePeriod::SecondHalf | GamePeriod::HalfTime => {
                renderer.in_game_display(&local_state);
                flag_renderer.draw();
            }
            GamePeriod::OvertimeFirstHalf
            | GamePeriod::OvertimeHalfTime
            | GamePeriod::OvertimeSecondHalf
            | GamePeriod::PreOvertime
            | GamePeriod::PreSuddenDeath
            | GamePeriod::SuddenDeath => {
                renderer.overtime_and_sudden_death_display(&local_state);
                flag_renderer.draw();
            }
        }
        next_frame().await;
    }
}

fn init_logging() {
    let args = Cli::parse();

    let log_level = match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let log_base_path = args.log_location.unwrap_or_else(|| {
        let mut path = directories::BaseDirs::new()
            .expect("Could not find a directory to store logs")
            .data_local_dir()
            .to_path_buf();
        path.push("uwh-overlay-logs");
        path
    });
    let mut log_path = log_base_path.clone();
    let mut archived_log_path = log_base_path.clone();
    log_path.push(format!("{APP_NAME}-log.txt"));
    archived_log_path.push(format!("{APP_NAME}-log-{{}}.txt.gz"));

    #[cfg(debug_assertions)]
    println!("Log path: {}", log_path.display());

    // Only log to the console in debug mode
    #[cfg(all(debug_assertions, not(target_os = "windows")))]
    let console_target = Target::Stderr;
    #[cfg(all(debug_assertions, target_os = "windows"))]
    let console_target = Target::Stdout; // Windows apps don't get a stderr handle
    #[cfg(debug_assertions)]
    let console = ConsoleAppender::builder()
        .target(console_target)
        .encoder(Box::new(PatternEncoder::new("[{d} {h({l:5})} {M}] {m}{n}")))
        .build();

    // Setup the file log roller
    let roller = FixedWindowRoller::builder()
        .build(
            archived_log_path.as_os_str().to_str().unwrap(),
            args.num_old_logs,
        )
        .unwrap();
    let file_policy = CompoundPolicy::new(
        Box::new(SizeTrigger::new(args.log_max_file_size)),
        Box::new(roller),
    );
    let file_appender = RollingFileAppender::builder()
        .append(true)
        .encoder(Box::new(PatternEncoder::new("[{d} {l:5} {M}] {m}{n}")))
        .build(log_path, Box::new(file_policy))
        .unwrap();

    // Setup the logging from all locations to use `LevelFilter::Error`
    let root = Root::builder().appender("file_appender");
    #[cfg(debug_assertions)]
    let root = root.appender("console");
    let root = root.build(LevelFilter::Error);

    // Setup the top level logging config
    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("file_appender", Box::new(file_appender)));

    #[cfg(debug_assertions)]
    let log_config = log_config.appender(Appender::builder().build("console", Box::new(console)));

    let log_config = log_config
        .logger(Logger::builder().build("overlay", log_level)) // Setup the logging from the refbox app to use `log_level`
        .build(root)
        .unwrap();

    log4rs::init_config(log_config).unwrap();
    log_panics::init();
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Overlay Program"),
        window_width: 3840,
        window_height: 1080,
        window_resizable: false,
        platform: miniquad::conf::Platform {
            linux_backend: miniquad::conf::LinuxBackend::X11WithWaylandFallback,
            ..Default::default()
        },
        ..Default::default()
    }
}
