//! Receives game snapshots from a refbox, the same way the overlay does.

use log::{debug, info, warn};
use std::{net::IpAddr, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpStream, sync::mpsc::UnboundedSender};
use uwh_common::game_snapshot::GameSnapshot;

/// A line longer than this without a newline isn't a JSON snapshot (e.g. the LED-panel feed on
/// the refbox's other port), so the buffer is dropped rather than allowed to grow.
const MAX_LINE: usize = 64 * 1024;

#[derive(Debug)]
pub enum RefboxEvent {
    Connected,
    Snapshot(Box<GameSnapshot>),
    /// Something arrived that isn't a game snapshot. Reported once per connection.
    Unreadable(String),
    Disconnected,
}

/// Connects to the refbox and forwards everything as `(court_index, event)`.
/// Reconnects forever; only returns once the receiver is gone.
pub async fn follow_refbox(
    court_index: usize,
    ip: IpAddr,
    port: u16,
    tx: UnboundedSender<(usize, RefboxEvent)>,
) {
    loop {
        let mut stream = match TcpStream::connect((ip, port)).await {
            Ok(stream) => stream,
            Err(e) => {
                debug!("Refbox at {ip}:{port} not reachable ({e}), retrying");
                if tx.is_closed() {
                    return;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        info!("Connected to refbox at {ip}:{port}");
        if tx.send((court_index, RefboxEvent::Connected)).is_err() {
            return;
        }

        // The refbox sends one JSON snapshot per line.
        let mut pending: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut reported_unreadable = false;
        let mut unreadable = |reason: String, tx: &UnboundedSender<(usize, RefboxEvent)>| {
            if !reported_unreadable {
                reported_unreadable = true;
                warn!("Ignoring unreadable data from {ip}:{port}: {reason}");
                let _ = tx.send((court_index, RefboxEvent::Unreadable(reason)));
            }
        };
        loop {
            let n = match stream.read(&mut chunk).await {
                Ok(0) => {
                    warn!("Refbox at {ip}:{port} closed the connection, reconnecting");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("Lost connection to refbox at {ip}:{port}: {e}, reconnecting");
                    break;
                }
            };
            pending.extend_from_slice(&chunk[..n]);
            while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = pending.drain(..=end).collect();
                let line = line.trim_ascii();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_slice::<GameSnapshot>(line) {
                    Ok(snapshot) => {
                        let event = RefboxEvent::Snapshot(Box::new(snapshot));
                        if tx.send((court_index, event)).is_err() {
                            return;
                        }
                    }
                    Err(e) => unreadable(e.to_string(), &tx),
                }
            }
            if pending.len() > MAX_LINE {
                pending.clear();
                unreadable("not line-based game data".to_string(), &tx);
            }
        }
        if tx.send((court_index, RefboxEvent::Disconnected)).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
