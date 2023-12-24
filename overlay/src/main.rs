use clap::Parser;
use coarsetime::Instant;
use crossbeam_channel::bounded;
use log::{warn, LevelFilter};
#[cfg(debug_assertions)]
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::{
    append::rolling_file::{
        policy::compound::{
            roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
        RollingFileAppender,
    },
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use macroquad::prelude::*;
use network::{GameData, StatePacket, TeamInfoRaw};
use std::{cmp::Ordering, str::FromStr};
use std::{net::IpAddr, path::PathBuf};
use uwh_common::game_snapshot::{GamePeriod, GameSnapshot, TimeoutSnapshot};

mod flag;
mod load_images;
mod network;
mod pages;

use load_images::{read_image_from_file, Texture};

const APP_NAME: &str = "overlay";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AppConfig {
    refbox_ip: IpAddr,
    refbox_port: u64,
    uwhscores_url: String,
    uwhportal_url: String,
    tournament_logo_path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refbox_ip: IpAddr::from_str("127.0.0.1").unwrap(),
            refbox_port: 8000,
            uwhscores_url: String::from("https://uwhscores.com"),
            uwhportal_url: String::from("https://api.uwhscores.prod.zmvp.host"),
            tournament_logo_path: PathBuf::new(),
        }
    }
}

pub struct State {
    snapshot: GameSnapshot,
    black: TeamInfo,
    white: TeamInfo,
    referees: Vec<Member>,
    game_id: u32,
    pool: String,
    start_time: String,
    half_play_duration: Option<u32>,
    sponsor_logo: Option<Texture>,
}

// TODO: Change this to return Result. We're not rn cause from_file_with_format
// panics anyways if image bytes is invalid
pub fn texture_from_image(image: network::Image) -> Texture {
    Texture {
        color: Texture2D::from_rgba8(image.0, image.1, &image.2),
        alpha: alphagen::on_raw_rgba8(image.0 as u32, image.1 as u32, image.2)
            .map(|bytes| Texture2D::from_file_with_format(&bytes, None))
            .expect("Failed to decode image"),
    }
}

impl State {
    fn update_state(&mut self, recieved_state: StatePacket) {
        if let Some(GameData {
            black,
            white,
            pool,
            start_time,
            sponsor_logo,
            referees,
            ..
        }) = recieved_state.data
        {
            self.black = TeamInfo::from(black);
            self.white = TeamInfo::from(white);
            self.start_time = start_time;
            self.sponsor_logo = sponsor_logo.map(texture_from_image);
            self.referees = referees.into_iter().map(Member::from).collect();
            self.pool = pool;
        }
        if let Some(game_id) = recieved_state.game_id {
            self.game_id = game_id;
        }
        self.snapshot = recieved_state.snapshot;
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
            picture: member_raw.picture.map(texture_from_image),
            geared_picture: member_raw.geared_picture.map(texture_from_image),
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
            flag: team_info_raw.flag.map(texture_from_image),
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

    let (tx, rx) = bounded::<StatePacket>(3);
    let mut tournament_logo_color_path = config.tournament_logo_path.clone();
    tournament_logo_color_path.push("color.png");
    let mut tournament_logo_alpha_path = config.tournament_logo_path.clone();
    tournament_logo_alpha_path.push("alpha.png");

    let net_worker = std::thread::spawn(|| {
        network::networking_thread(tx, config);
    });

    let mut assets = load_images::Textures::default();

    let tournament_logo_color = match read_image_from_file(tournament_logo_color_path.as_path()) {
        Ok(texture) => Some(texture),
        Err(e) => {
            warn!(
                "Failed to read tournament logo color file: {} : {e}",
                tournament_logo_color_path.display()
            );
            None
        }
    };

    let tournament_logo_alpha = match read_image_from_file(tournament_logo_alpha_path.as_path()) {
        Ok(texture) => Some(texture),
        Err(e) => {
            warn!(
                "Failed to read tournament logo alpha file: {} : {e}",
                tournament_logo_alpha_path.display()
            );
            None
        }
    };

    assets.tournament_logo = tournament_logo_color
        .and_then(|color| tournament_logo_alpha.map(|alpha| Texture { alpha, color }));

    let mut local_state: State = State {
        snapshot: GameSnapshot {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 600,
            ..Default::default()
        },

        black: TeamInfo::with_name("BLACK"),
        referees: Vec::new(),
        white: TeamInfo::with_name("WHITE"),
        game_id: 0,
        pool: String::new(),
        start_time: String::new(),
        half_play_duration: None,
        sponsor_logo: None,
    };

    let mut renderer = pages::PageRenderer {
        animation_register0: Instant::now(),
        animation_register1: Instant::now(),
        animation_register2: Instant::now(),
        animation_register3: false,
        assets,
        last_snapshot_timeout: TimeoutSnapshot::None,
    };

    let mut flag_renderer = flag::Renderer::new();
    unsafe {
        get_internal_gl().quad_context.show_mouse(false);
    }

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
        ..Default::default()
    }
}
