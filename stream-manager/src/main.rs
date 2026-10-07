use clap::{Parser, Subcommand};
use log::{LevelFilter, error, info};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use std::path::{Path, PathBuf};

mod app;
mod config;
mod google_auth;
mod live;
mod portal;
mod prepare;
mod quota;
mod recovery;
mod refbox;
mod switcher;
mod title_sync;
mod vmix;
mod web;
mod youtube;

use config::Config;
use portal::{playlist_title, video_title};
use youtube::YouTube;

/// Error type used throughout; thread-safe so errors can come back from background tasks.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

const APP_NAME: &str = "stream_manager";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Config file to use (default: Documents\stream-manager\config.toml)
    #[clap(long, short)]
    config: Option<PathBuf>,

    #[clap(long, short, action(clap::ArgAction::Count))]
    /// Increase the log verbosity
    verbose: u8,

    /// Don't open the control page in a browser on start
    #[clap(long)]
    no_browser: bool,

    /// What to do. Without a command, the control page starts (this is what a double-click does).
    #[clap(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Start the control page (the default)
    Serve,
    /// Show the playlists and video titles that would be created from the portal schedule
    Plan,
    /// Sign in to YouTube once (opens Google's sign-in page in the browser)
    Connect,
    /// Check the YouTube connection and that each court's two stream keys exist
    CheckYoutube,
    /// Create or update the playlists and scheduled videos for one tournament day
    Prepare {
        /// Tournament day number, as shown by `plan`
        #[clap(long)]
        day: usize,
        /// Only this court (default: every court in the config)
        #[clap(long)]
        court: Option<String>,
        /// Only the first N games of each playlist (handy for testing)
        #[clap(long)]
        limit: Option<usize>,
    },
    /// Show what YouTube reports for the videos created for this event (read-only)
    Videos,
    /// Permanently delete the videos and playlists created for this event (for test clean-up)
    Cleanup,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    init_logging(args.verbose);

    let config_path = args.config.unwrap_or_else(config::default_config_path);
    let command = args.command.unwrap_or(CliCommand::Serve);
    let serving = matches!(command, CliCommand::Serve);
    if let Err(e) = run(command, &config_path, !args.no_browser).await {
        error!("{e}");
        if serving {
            // Keep a double-clicked window open long enough to read the problem.
            eprintln!("\nPress Enter to close this window.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
}

async fn run(command: CliCommand, config_path: &Path, open_browser: bool) -> Result<(), BoxError> {
    let config = load_config(config_path)?;
    if let CliCommand::Serve = command {
        let app = app::App::new(config_path.to_path_buf(), config);
        app.start_refbox_connections();
        let plan_app = std::sync::Arc::clone(&app);
        // Recovery needs the schedule, so it runs once the schedule has loaded.
        tokio::spawn(async move {
            plan_app.refresh_plan().await;
            plan_app.recover_live_videos().await;
        });
        // Every 10 minutes, keeps running courts' upcoming titles in step with the portal.
        let sync_app = std::sync::Arc::clone(&app);
        tokio::spawn(async move { sync_app.run_title_sync().await });
        return web::serve(app, open_browser).await;
    }

    if config.event_slug.is_empty() {
        return Err(format!(
            "Choose an event first (Settings tab, or `event_slug` in {})",
            config_path.display()
        )
        .into());
    }
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let client_file = config_dir.join(&config.client_secret_file);
    let token_file = config_dir.join(app::TOKEN_FILE);
    let state_file = prepare::state_path(config_dir, &config.event_slug);
    // The CLI uses the same allowance as the control page, so it counts in the same ledger.
    let ledger_file = config_dir.join(quota::LEDGER_FILE);
    let youtube = || -> Result<YouTube, BoxError> {
        Ok(YouTube::new(
            google_auth::GoogleAuth::load(&client_file, &token_file)?,
            Some(ledger_file.clone()),
        ))
    };

    match command {
        CliCommand::Serve => Ok(()),
        CliCommand::Plan => show_plan(&config).await,
        CliCommand::Connect => {
            google_auth::connect(&client_file, &token_file).await?;
            let mut yt = youtube()?;
            println!(
                "Connected to YouTube channel: {}",
                yt.my_channel_title().await?
            );
            Ok(())
        }
        CliCommand::CheckYoutube => check_youtube(&config, &mut youtube()?).await,
        CliCommand::Prepare { day, court, limit } => {
            let plan = portal::fetch_event_plan(&config.portal_url, &config.event_slug).await?;
            let mut yt = youtube()?;
            println!("YouTube channel: {}", yt.my_channel_title().await?);
            let selection = prepare::Selection {
                day,
                courts: court.into_iter().collect(),
                limit,
            };
            prepare::run_cli(&config, &plan, &mut yt, &state_file, &selection).await
        }
        CliCommand::Videos => show_videos(&config, &mut youtube()?, &state_file).await,
        CliCommand::Cleanup => {
            let mut yt = youtube()?;
            println!("YouTube channel: {}", yt.my_channel_title().await?);
            prepare::cleanup_cli(&mut yt, &state_file, &config.event_slug).await
        }
    }
}

async fn show_videos(config: &Config, yt: &mut YouTube, state_file: &Path) -> Result<(), BoxError> {
    let state = prepare::load_state(state_file, &config.event_slug)?;
    let ids: Vec<&str> = state
        .videos
        .values()
        .map(|v| v.broadcast_id.as_str())
        .collect();
    if ids.is_empty() {
        println!("No videos recorded for {}.", config.event_slug);
        return Ok(());
    }
    let mut found = Vec::new();
    for chunk in ids.chunks(50) {
        found.extend(yt.broadcast_statuses(chunk).await?);
    }
    for (game, video) in &state.videos {
        match found.iter().find(|f| f[0] == video.broadcast_id) {
            Some([id, title, life, privacy, stream]) => println!(
                "Game {game}: {life}, {privacy}, stream {stream}\n  {title}\n  https://youtu.be/{id}"
            ),
            None => println!("Game {game}: not found on YouTube (deleted?)"),
        }
    }
    println!("Used {} units.", yt.units_used);
    Ok(())
}

async fn check_youtube(config: &Config, yt: &mut YouTube) -> Result<(), BoxError> {
    println!(
        "Connected to YouTube channel: {}",
        yt.my_channel_title().await?
    );
    let streams = yt.list_streams().await?;
    let mut all_ok = true;
    for court in &config.courts {
        match prepare::court_streams(court, &streams) {
            Ok(pair) => {
                for s in pair {
                    let sending = if s.stream_status == "active" {
                        "yes"
                    } else {
                        "no"
                    };
                    println!(
                        "  Court {}: stream key \"{}\" found (vMix sending: {sending})",
                        court.name, s.title
                    );
                }
            }
            Err(e) => {
                all_ok = false;
                println!("  {e}");
            }
        }
    }
    if !all_ok {
        let names: Vec<_> = streams.iter().map(|s| format!("\"{}\"", s.title)).collect();
        println!("Stream keys on this channel: {}", names.join(", "));
    }
    println!("Used {} units.", yt.units_used);
    Ok(())
}

fn init_logging(verbose: u8) {
    let log_level = match verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    let console = ConsoleAppender::builder()
        .target(Target::Stderr)
        .encoder(Box::new(PatternEncoder::new(
            "[{d(%H:%M:%S)} {h({l:5})}] {m}{n}",
        )))
        .build();
    let log_config = LogConfig::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .logger(Logger::builder().build(APP_NAME, log_level))
        .build(Root::builder().appender("console").build(LevelFilter::Warn));
    match log_config {
        Ok(log_config) => {
            if log4rs::init_config(log_config).is_ok() {
                log_panics::init();
            }
        }
        Err(e) => eprintln!("Couldn't set up logging: {e}"),
    }
}

/// Loads the settings, creating a default settings file on first run.
fn load_config(path: &Path) -> Result<Config, BoxError> {
    if !path.exists() {
        confy::store_path(path, Config::default())?;
        info!("Created settings file {}", path.display());
    }
    let config: Config = confy::load_path(path)?;
    config
        .validate()
        .map_err(|e| format!("{e} (in {})", path.display()))?;
    Ok(config)
}

async fn show_plan(config: &Config) -> Result<(), BoxError> {
    let plan = portal::fetch_event_plan(&config.portal_url, &config.event_slug).await?;
    println!("Event: {}  ({} games)\n", plan.event_name, plan.games.len());
    for ((day, court), games) in plan.playlists() {
        let streamed = config.courts.iter().any(|c| c.name == court);
        let note = if streamed {
            ""
        } else {
            "   (court not in config)"
        };
        println!(
            "Playlist \"{}\" — {} videos{note}",
            playlist_title(day, &court),
            games.len()
        );
        for game in games {
            let start = game
                .start
                .format(time::macros::format_description!("[hour]:[minute]"))
                .unwrap_or_default();
            println!("  {start}  {}", video_title(&plan.event_name, game));
        }
        println!();
    }
    Ok(())
}
