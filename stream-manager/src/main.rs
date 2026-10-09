use clap::{Parser, Subcommand};
use log::{LevelFilter, error, info};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use std::path::{Path, PathBuf};

mod access;
mod app;
mod companion;
mod config;
mod google_auth;
mod live;
mod portal;
mod prepare;
mod quota;
mod recovery;
mod refbox;
mod switcher;
mod thumbnail;
mod title_sync;
mod vmix;
// Prepare, cleanup and the settings card use it once they put links on the portal.
#[cfg_attr(not(test), allow(dead_code))]
mod watch_links;
mod web;
mod youtube;

use config::Config;
use portal::{playlist_title, video_title};
use youtube::YouTube;

/// Error type used throughout; thread-safe so errors can come back from background tasks.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// How long a request to YouTube, Google sign-in or the portal may take before it is given up,
/// so a server that never answers can't hold up a court's switches.
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// The web client for YouTube, Google sign-in and the portal.
pub fn http_client() -> Result<reqwest::Client, BoxError> {
    Ok(reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?)
}

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
    /// Save the thumbnails Prepare would upload as picture files (doesn't touch YouTube)
    Thumbnails {
        /// Folder to save them in
        #[clap(long, default_value = "thumbnails")]
        out: PathBuf,
        /// Only this tournament day (default: every day)
        #[clap(long)]
        day: Option<usize>,
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

/// Whether `command` works on one event, and so needs one chosen. Connecting and checking
/// YouTube work on a fresh install.
fn needs_event(command: &CliCommand) -> bool {
    !matches!(
        command,
        CliCommand::Serve | CliCommand::Connect | CliCommand::CheckYoutube
    )
}

async fn run(command: CliCommand, config_path: &Path, open_browser: bool) -> Result<(), BoxError> {
    let config = load_config(config_path)?;
    if let CliCommand::Serve = command {
        let app = app::App::new(config_path.to_path_buf(), config);
        app.start_refbox_connections();
        let plan_app = std::sync::Arc::clone(&app);
        // Recovery needs the schedule, so it runs once the schedule has loaded. If that fails,
        // it runs after the next schedule load that works.
        tokio::spawn(async move { plan_app.refresh_plan_and_recover().await });
        // Every 10 minutes, keeps running courts' upcoming titles in step with the portal.
        let sync_app = std::sync::Arc::clone(&app);
        tokio::spawn(async move { sync_app.run_title_sync().await });
        // Every second, keeps the Stream Deck's live status in Companion up to date.
        let companion_app = std::sync::Arc::clone(&app);
        tokio::spawn(async move { companion_app.run_companion_sync().await });
        return web::serve(app, open_browser).await;
    }

    if needs_event(&command) && config.event_slug.is_empty() {
        return Err(format!(
            "Choose an event first (Settings tab, or `event_slug` in {})",
            config_path.display()
        )
        .into());
    }
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let client_file = config_dir.join(&config.client_secret_file);
    let token_file = config_dir.join(app::TOKEN_FILE);
    // Only for the commands that work on one event (checked above).
    let state_file = || prepare::state_path(config_dir, &config.event_slug);
    // The CLI uses the same allowance as the control page, so it counts in the same ledger.
    let ledger = std::sync::Arc::new(quota::LedgerFile::open(config_dir.join(quota::LEDGER_FILE)));
    let youtube = || -> Result<YouTube, BoxError> {
        YouTube::new(
            google_auth::GoogleAuth::load(&client_file, &token_file)?,
            Some(std::sync::Arc::clone(&ledger)),
        )
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
            prepare::run_cli(&config, &plan, &mut yt, &state_file()?, &selection).await
        }
        CliCommand::Thumbnails { out, day } => save_thumbnails(&config, &out, day).await,
        CliCommand::Videos => show_videos(&config, &mut youtube()?, &state_file()?).await,
        CliCommand::Cleanup => {
            let mut yt = youtube()?;
            println!("YouTube channel: {}", yt.my_channel_title().await?);
            prepare::cleanup_cli(&mut yt, &state_file()?, &config.event_slug).await
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
    let mut config: Config = confy::load_path(path)?;
    config
        .validate()
        .map_err(|e| format!("{e} (in {})", path.display()))?;
    if config.button_key.is_empty() {
        config.button_key = access::new_button_key()
            .map_err(|e| format!("Couldn't create the Stream Deck button key: {e}"))?;
        app::save_config(path, &config)?;
        info!("Created the Stream Deck button key");
    }
    if config.stream_manager_id.is_empty() {
        let id = access::new_stream_manager_id()
            .map_err(|e| format!("Couldn't create the Stream Manager ID: {e}"))?;
        config.stream_manager_id = id.clone();
        app::save_config(path, &config)?;
        info!("Created the Stream Manager ID {id}");
    }
    Ok(config)
}

async fn save_thumbnails(config: &Config, out: &Path, day: Option<usize>) -> Result<(), BoxError> {
    let plan = portal::fetch_event_plan(&config.portal_url, &config.event_slug).await?;
    std::fs::create_dir_all(out)?;
    let mut thumbnails = prepare::Thumbnails::default();
    let mut count = 0;
    for game in plan.games.iter().filter(|g| day.is_none_or(|d| g.day == d)) {
        let (jpeg, _) = thumbnails
            .draw(&plan, game, &mut |line| info!("{line}"))
            .await?;
        let number: String = game
            .number
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect();
        let name = format!("day{}-court{}-game{number}.jpg", game.day, game.court);
        std::fs::write(out.join(name), jpeg)?;
        count += 1;
    }
    println!("Saved {count} thumbnail(s) in {}", out.display());
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connecting_and_checking_youtube_need_no_event() {
        assert!(!needs_event(&CliCommand::Connect));
        assert!(!needs_event(&CliCommand::CheckYoutube));
        assert!(needs_event(&CliCommand::Plan));
        assert!(needs_event(&CliCommand::Videos));
        assert!(needs_event(&CliCommand::Cleanup));
        assert!(needs_event(&CliCommand::Prepare {
            day: 1,
            court: None,
            limit: None,
        }));
    }

    #[test]
    fn the_first_start_saves_a_stream_manager_id_that_then_stays() {
        let dir =
            std::env::temp_dir().join(format!("stream-manager-main-id-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let first = load_config(&path).unwrap().stream_manager_id;
        assert_eq!(first.len(), 6, "{first}");
        assert!(first.chars().all(|c| c.is_ascii_digit()), "{first}");
        let saved: Config = confy::load_path(&path).unwrap();
        assert_eq!(saved.stream_manager_id, first);
        assert_eq!(load_config(&path).unwrap().stream_manager_id, first);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
