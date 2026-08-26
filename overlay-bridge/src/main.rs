use std::{net::SocketAddr, sync::Arc};

use clap::Parser;
use overlay_bridge::{
    config,
    feed::Supervisor,
    server::{self, AppState},
};
use reqwest::Client;
use tokio::sync::{Notify, mpsc};

/// Reads a refbox's live game feed and serves it to vMix (or any other third party) as JSON
/// tables over HTTP, surviving refbox dropouts and Portal outages without the served picture
/// ever going wrong.
///
/// This binary is deliberately thin: CLI parsing, resolving settings (CLI beats a stored value
/// beats a built-in default -- see `config`'s module doc), wiring the feed supervisor, the Portal
/// directory and the HTTP server together, and the tokio runtime. Everything else -- the feed
/// reader, the live-picture/clock logic, the Portal client, the served table shapes, the HTTP
/// routes themselves, and the status page -- lives in the library crate (`overlay_bridge::*`) so
/// it is testable without a real refbox, network, or clock.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Hostname or IP address of the refbox to connect to. Overrides the saved setting when
    /// passed; otherwise the last address used is remembered (design spec §5.3), falling back to
    /// 127.0.0.1 if nothing has ever been saved.
    #[clap(long)]
    refbox_host: Option<String>,

    /// TCP port the refbox serves its game-state feed on. Same precedence as `--refbox-host`.
    #[clap(long)]
    refbox_port: Option<u16>,

    /// URL of the uwhportal instance to resolve team names and rosters from. No credentials are
    /// ever sent -- see `portal`'s module doc.
    #[clap(long, default_value = "https://api.uwhportal.com")]
    portal_url: String,

    /// Port the bridge's own HTTP server listens on, for vMix and any other poller. Not 8088 --
    /// that is vMix's own web controller and would collide on the same PC. Overrides the saved
    /// setting when passed; otherwise the last value used is remembered, falling back to 8099.
    #[clap(long)]
    port: Option<u16>,

    /// Whether the white team is drawn on the physical right of the pool. The refbox's own feed
    /// does not carry this -- it is a camera/venue setting, chosen once per session (design spec
    /// §5.2). Bare `--white-on-right` means true; `--white-on-right=false` explicitly overrides a
    /// saved `true` back to false. Absent entirely, the saved value is remembered, falling back
    /// to false.
    #[clap(long, num_args = 0..=1, default_missing_value = "true")]
    white_on_right: Option<bool>,

    /// The court label -- the other setting the refbox feed cannot supply (design spec §5.2).
    /// Same precedence as the settings above; falls back to empty (not set) if nothing is saved.
    #[clap(long)]
    court: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let stored = config::load();

    let refbox_host = config::resolve(
        cli.refbox_host.clone(),
        stored.refbox_host.clone(),
        config::DEFAULT_REFBOX_HOST.to_string(),
    );
    let refbox_port = config::resolve(
        cli.refbox_port,
        stored.refbox_port,
        config::DEFAULT_REFBOX_PORT,
    );
    let port = config::resolve(cli.port, stored.port, config::DEFAULT_PORT);
    let white_on_right = config::resolve(cli.white_on_right, stored.white_on_right, false);
    let court = config::resolve(cli.court.clone(), stored.court.clone(), String::new());

    // Persist whatever was actually resolved this run -- including a value that came from the
    // built-in default -- so it becomes "what was last used" for the next run with no flags at
    // all (design spec §5.3: "the last address used is remembered").
    config::store(&config::Settings {
        refbox_host: Some(refbox_host.clone()),
        refbox_port: Some(refbox_port),
        port: Some(port),
        white_on_right: Some(white_on_right),
        court: Some(court.clone()),
    });

    println!("Connecting to refbox at {refbox_host}:{refbox_port}...");

    let state = Arc::new(AppState::new(white_on_right).with_operator_info(
        refbox_host.clone(),
        refbox_port,
        court,
    ));

    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(Supervisor::run(
        (refbox_host, refbox_port),
        tx,
        state.connection_handle(),
    ));

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

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
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
    use overlay_bridge::config;

    use super::*;

    // CLI parsing itself now only ever produces `None` (not passed) or `Some(value)` (passed) --
    // no `default_value` any more, so an explicit pass can be told apart from "nothing typed"
    // (see `config`'s module doc for why that distinction has to exist). The *effective* value
    // used when nothing was typed comes from `config::resolve`, exercised alongside the parse in
    // each test below so both halves of the precedence rule stay covered together.

    #[test]
    fn the_http_port_is_not_set_by_default_but_resolves_to_8099_never_8088() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.port, None);
        assert_eq!(config::resolve(cli.port, None, config::DEFAULT_PORT), 8099);
    }

    #[test]
    fn the_http_port_is_configurable() {
        let cli = Cli::try_parse_from(["overlay-bridge", "--port", "9001"])
            .expect("--port should be accepted");
        assert_eq!(cli.port, Some(9001));
    }

    #[test]
    fn white_on_right_is_not_set_by_default_but_resolves_to_false() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.white_on_right, None);
        assert!(!config::resolve(cli.white_on_right, None, false));
    }

    #[test]
    fn white_on_right_flag_alone_sets_it_true() {
        let cli = Cli::try_parse_from(["overlay-bridge", "--white-on-right"])
            .expect("--white-on-right should be accepted");
        assert_eq!(cli.white_on_right, Some(true));
    }

    #[test]
    fn white_on_right_flag_accepts_an_explicit_false_to_override_a_saved_true() {
        // Without this, an operator whose saved setting is `true` from a previous session would
        // have no way to pass `false` this run -- a bare boolean flag can only ever *set* true by
        // being present, never explicitly clear a stored value back to false.
        let cli = Cli::try_parse_from(["overlay-bridge", "--white-on-right=false"])
            .expect("--white-on-right=false should be accepted");
        assert_eq!(cli.white_on_right, Some(false));
    }

    #[test]
    fn refbox_host_and_port_are_not_set_by_default_but_resolve_to_the_built_in_defaults() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.refbox_host, None);
        assert_eq!(cli.refbox_port, None);
        assert_eq!(
            config::resolve(
                cli.refbox_host,
                None,
                config::DEFAULT_REFBOX_HOST.to_string()
            ),
            "127.0.0.1"
        );
        assert_eq!(
            config::resolve(cli.refbox_port, None, config::DEFAULT_REFBOX_PORT),
            8000
        );
    }

    #[test]
    fn court_is_not_set_by_default_but_resolves_to_empty() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.court, None);
        assert_eq!(
            config::resolve(cli.court, None, String::new()),
            String::new()
        );
    }

    #[test]
    fn the_portal_url_defaults_to_the_production_portal() {
        let cli = Cli::try_parse_from(["overlay-bridge"]).expect("no required args");
        assert_eq!(cli.portal_url, "https://api.uwhportal.com");
    }
}
