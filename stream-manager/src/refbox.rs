//! Receives game snapshots from a refbox, the same way the overlay does.

use log::{debug, info, warn};
use std::{net::IpAddr, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
    sync::mpsc::UnboundedSender,
};
use uwh_common::game_snapshot::GameSnapshot;

#[derive(Debug)]
pub enum RefboxEvent {
    Connected,
    Snapshot(Box<GameSnapshot>),
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
        let stream = match TcpStream::connect((ip, port)).await {
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
        let mut lines = BufReader::new(stream).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => match serde_json::from_str::<GameSnapshot>(&line) {
                    Ok(snapshot) => {
                        let event = RefboxEvent::Snapshot(Box::new(snapshot));
                        if tx.send((court_index, event)).is_err() {
                            return;
                        }
                    }
                    Err(e) => warn!("Ignoring unreadable snapshot from {ip}:{port}: {e}"),
                },
                Ok(None) => {
                    warn!("Refbox at {ip}:{port} closed the connection, reconnecting");
                    break;
                }
                Err(e) => {
                    warn!("Lost connection to refbox at {ip}:{port}: {e}, reconnecting");
                    break;
                }
            }
        }
        if tx.send((court_index, RefboxEvent::Disconnected)).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
