//! Receives game snapshots from a refbox, the same way the overlay does.

use log::{debug, info, warn};
use std::{net::IpAddr, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpStream, sync::mpsc::UnboundedSender};
use uwh_common::game_snapshot::GameSnapshot;

/// A line longer than this without a newline isn't a JSON snapshot (e.g. the LED-panel feed on
/// the refbox's other port), so the buffer is dropped rather than allowed to grow.
const MAX_LINE: usize = 64 * 1024;

/// While a clock runs (a game or the break countdown), the refbox sends a snapshot every
/// second. This long without one on an open connection is reported as [`RefboxEvent::Silent`],
/// e.g. after a Wi-Fi drop that never closed the connection.
pub const SILENCE: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum RefboxEvent {
    Connected,
    Snapshot(Box<GameSnapshot>),
    /// Something arrived that isn't a game snapshot. Reported once per connection.
    Unreadable(String),
    /// Nothing has arrived for [`SILENCE`]. Sent once per silence; the next snapshot ends it.
    /// The refbox also goes quiet while its clock is stopped, so this alone isn't a fault.
    Silent,
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
    follow_refbox_with(court_index, ip, port, tx, SILENCE).await;
}

/// [`follow_refbox`] with the silence reported after `silence` (shorter in tests).
async fn follow_refbox_with(
    court_index: usize,
    ip: IpAddr,
    port: u16,
    tx: UnboundedSender<(usize, RefboxEvent)>,
    silence: Duration,
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
        let mut reported_silent = false;
        let mut unreadable = |reason: String, tx: &UnboundedSender<(usize, RefboxEvent)>| {
            if !reported_unreadable {
                reported_unreadable = true;
                warn!("Ignoring unreadable data from {ip}:{port}: {reason}");
                let _ = tx.send((court_index, RefboxEvent::Unreadable(reason)));
            }
        };
        loop {
            let n = match tokio::time::timeout(silence, stream.read(&mut chunk)).await {
                Err(_) => {
                    if !reported_silent {
                        reported_silent = true;
                        debug!("Nothing from refbox at {ip}:{port} for {silence:?}");
                        if tx.send((court_index, RefboxEvent::Silent)).is_err() {
                            return;
                        }
                    }
                    continue;
                }
                Ok(Ok(0)) => {
                    warn!("Refbox at {ip}:{port} closed the connection, reconnecting");
                    break;
                }
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
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
                        reported_silent = false;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tokio::{io::AsyncWriteExt, net::TcpListener, sync::mpsc};

    async fn next(rx: &mut mpsc::UnboundedReceiver<(usize, RefboxEvent)>) -> RefboxEvent {
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("an event in time")
            .expect("the reader is still running")
            .1
    }

    #[tokio::test]
    async fn a_quiet_open_connection_is_reported_once_until_the_next_snapshot() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let reader = tokio::spawn(follow_refbox_with(
            0,
            Ipv4Addr::LOCALHOST.into(),
            port,
            tx,
            Duration::from_millis(100),
        ));
        let (mut refbox, _) = listener.accept().await.unwrap();
        assert!(matches!(next(&mut rx).await, RefboxEvent::Connected));
        let line = serde_json::to_string(&GameSnapshot::default()).unwrap() + "\n";
        refbox.write_all(line.as_bytes()).await.unwrap();
        assert!(matches!(next(&mut rx).await, RefboxEvent::Snapshot(_)));
        // Quiet for several silence periods: reported once, the connection stays open.
        assert!(matches!(next(&mut rx).await, RefboxEvent::Silent));
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert!(rx.try_recv().is_err());
        // A snapshot ends the silence; the next one is reported again.
        refbox.write_all(line.as_bytes()).await.unwrap();
        assert!(matches!(next(&mut rx).await, RefboxEvent::Snapshot(_)));
        assert!(matches!(next(&mut rx).await, RefboxEvent::Silent));
        reader.abort();
    }
}
