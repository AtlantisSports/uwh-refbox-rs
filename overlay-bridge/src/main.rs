use clap::Parser;
use futures::StreamExt;
use tokio::net::TcpStream;

mod feed;

use feed::SnapshotReader;

/// Reads a refbox's live game feed.
///
/// This is the crate skeleton: it proves the feed reader against a real refbox by connecting once
/// and printing each snapshot as it arrives. The HTTP server that republishes the game to vMix,
/// reconnect-on-dropout, and everything else described in the design arrive in later tasks.
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
    let stream = match TcpStream::connect((cli.refbox_host.as_str(), cli.refbox_port)).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Could not connect to refbox: {e}");
            std::process::exit(1);
        }
    };
    println!("Connected. Streaming game snapshots (Ctrl+C to stop)...");

    let mut snapshots = SnapshotReader::new(stream);
    while let Some(result) = snapshots.next().await {
        match result {
            Ok(snapshot) => println!("{snapshot:?}"),
            Err(e) => eprintln!("Could not read a snapshot from the feed: {e}"),
        }
    }

    println!("Refbox closed the connection.");
}
