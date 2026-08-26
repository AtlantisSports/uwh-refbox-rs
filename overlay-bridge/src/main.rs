use clap::Parser;
use overlay_bridge::feed::Supervisor;
use tokio::sync::mpsc;

/// Reads a refbox's live game feed.
///
/// This is the crate skeleton: it proves the connection supervisor against a real refbox by
/// connecting, printing each snapshot as it arrives, and reconnecting -- with TCP keepalive so a
/// silently-vanished refbox is noticed instead of hanging forever -- if the connection is ever
/// lost. The HTTP server that republishes the game to vMix, and everything else described in the
/// design, arrive in later tasks.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Hostname or IP address of the refbox to connect to.
    #[clap(long, default_value = "127.0.0.1")]
    refbox_host: String,

    /// TCP port the refbox serves its game-state feed on.
    #[clap(long, default_value = "8000")]
    refbox_port: u16,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!(
        "Connecting to refbox at {}:{}...",
        cli.refbox_host, cli.refbox_port
    );

    let (tx, mut rx) = mpsc::unbounded_channel();
    tokio::spawn(Supervisor::run((cli.refbox_host, cli.refbox_port), tx));

    println!("Streaming game snapshots (Ctrl+C to stop)...");
    while let Some(snapshot) = rx.recv().await {
        println!("{snapshot:?}");
    }
}
