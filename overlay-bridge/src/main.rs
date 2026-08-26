use std::{net::SocketAddr, sync::Arc};

use clap::Parser;
use overlay_bridge::{
    feed::Supervisor,
    server::{self, AppState},
};
use reqwest::Client;
use tokio::sync::{Notify, mpsc};

/// Reads a refbox's live game feed and serves it to vMix (or any other third party) as JSON
/// tables over HTTP, surviving refbox dropouts and Portal outages without the served picture
/// ever going wrong.
///
/// This binary is deliberately thin: CLI parsing, wiring the feed supervisor, the Portal
/// directory and the HTTP server together, and the tokio runtime. Everything else -- the feed
/// reader, the live-picture/clock logic, the Portal client, the served table shapes, and the
/// HTTP routes themselves -- lives in the library crate (`overlay_bridge::*`) so it is testable
/// without a real refbox, network, or clock.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Hostname or IP address of the refbox to connect to.
    #[clap(long, default_value = "127.0.0.1")]
    refbox_host: String,

    /// TCP port the refbox serves its game-state feed on.
    #[clap(long, default_value = "8000")]
    refbox_port: u16,

    /// URL of the uwhportal instance to resolve team names and rosters from. No credentials are
    /// ever sent -- see `portal`'s module doc.
    #[clap(long, default_value = "https://api.uwhportal.com")]
    portal_url: String,

    /// Port the bridge's own HTTP server listens on, for vMix and any other poller. Not 8088 --
    /// that is vMix's own web controller and would collide on the same PC.
    #[clap(long, default_value = "8099")]
    port: u16,

    /// Whether the white team is drawn on the physical right of the pool. The refbox's own feed
    /// does not carry this -- it is a camera/venue setting, chosen once per session. Persisting
    /// it across runs is Task 7's job, not this one's.
    #[clap(long)]
    white_on_right: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!(
        "Connecting to refbox at {}:{}...",
        cli.refbox_host, cli.refbox_port
    );

    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(Supervisor::run(
        (cli.refbox_host.clone(), cli.refbox_port),
        tx,
    ));

    let state = Arc::new(AppState::new(cli.white_on_right));
    let refresh_notify = Arc::new(Notify::new());
    let client = Client::new();

    tokio::spawn(server::consume_snapshots(
        Arc::clone(&state),
        rx,
        client,
        cli.portal_url.clone(),
        Arc::clone(&refresh_notify),
    ));
    tokio::spawn(server::refresh_portal_loop(
        Arc::clone(&state),
        refresh_notify,
        server::PORTAL_REFRESH_INTERVAL,
    ));

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("could not start the bridge's HTTP server on {addr}: {e}");
            return;
        }
    };
    println!("Serving vMix tables at http://{addr}/ (Ctrl+C to stop)...");

    if let Err(e) = axum::serve(listener, server::router(state)).await {
        eprintln!("the bridge's HTTP server stopped unexpectedly: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_http_port_defaults_to_8099_never_8088() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.port, 8099);
    }

    #[test]
    fn the_http_port_is_configurable() {
        let cli = Cli::try_parse_from(["overlay-bridge", "--port", "9001"])
            .expect("--port should be accepted");
        assert_eq!(cli.port, 9001);
    }

    #[test]
    fn white_on_right_defaults_to_false() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert!(!cli.white_on_right);
    }

    #[test]
    fn white_on_right_flag_sets_it_true() {
        let cli = Cli::try_parse_from(["overlay-bridge", "--white-on-right"])
            .expect("--white-on-right should be accepted");
        assert!(cli.white_on_right);
    }

    #[test]
    fn the_portal_url_defaults_to_the_production_portal() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.portal_url, "https://api.uwhportal.com");
    }
}
