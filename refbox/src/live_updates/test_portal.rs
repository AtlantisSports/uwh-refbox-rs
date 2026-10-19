//! A fake portal on a loopback port, shared by the live and stats senders' tests.

use crate::config::AccessKey;
use crate::portal_manager::{SharedAccessKeys, SharedUwhPortalClient, SiteAccessKeys};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::Instant,
};
use uwh_common::uwhportal::{UwhPortalClient, schedule::EventId};

pub(super) const KEY: &str = "KEY-LIVE";

/// How the fake portal answers the request with this index (0-based).
#[derive(Clone, Copy)]
pub(super) enum Reply {
    /// Answer `204 No Content` after holding the request this long.
    After(Duration),
    /// Answer at once with this status line and a short body.
    Status(&'static str),
    /// Never answer.
    Hang,
}

pub(super) struct Recorded {
    pub(super) at: Instant,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) authorization: Option<String>,
    pub(super) body: String,
}

pub(super) struct Portal {
    pub(super) site: String,
    requests: mpsc::UnboundedReceiver<Recorded>,
    pub(super) max_in_flight: Arc<AtomicUsize>,
}

impl Portal {
    /// The next request, if one arrives within `within`.
    pub(super) async fn next(&mut self, within: Duration) -> Option<Recorded> {
        tokio::time::timeout(within, self.requests.recv())
            .await
            .ok()
            .flatten()
    }

    /// Every request that arrives within `within`.
    pub(super) async fn all_within(&mut self, within: Duration) -> Vec<Recorded> {
        let deadline = Instant::now() + within;
        let mut got = Vec::new();
        while let Ok(Some(r)) = tokio::time::timeout_at(deadline, self.requests.recv()).await {
            got.push(r);
        }
        got
    }
}

/// A fake portal on a loopback port. `reply` decides, per request (its index and path), how it
/// answers.
pub(super) async fn portal(reply: impl Fn(usize, &str) -> Reply + Send + Sync + 'static) -> Portal {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let site = format!("http://{}", listener.local_addr().unwrap());
    let (tx, requests) = mpsc::unbounded_channel();
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let count = Arc::new(AtomicUsize::new(0));
    let reply = Arc::new(reply);
    let max = max_in_flight.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let (tx, reply, count, in_flight, max) = (
                tx.clone(),
                reply.clone(),
                count.clone(),
                in_flight.clone(),
                max.clone(),
            );
            tokio::spawn(async move {
                serve(stream, tx, reply, count, in_flight, max).await;
            });
        }
    });
    Portal {
        site,
        requests,
        max_in_flight,
    }
}

/// Serve one connection, request after request (the client keeps connections open).
async fn serve(
    stream: TcpStream,
    tx: mpsc::UnboundedSender<Recorded>,
    reply: Arc<impl Fn(usize, &str) -> Reply>,
    count: Arc<AtomicUsize>,
    in_flight: Arc<AtomicUsize>,
    max: Arc<AtomicUsize>,
) {
    let mut stream = BufReader::new(stream);
    loop {
        let mut request_line = String::new();
        if stream.read_line(&mut request_line).await.unwrap_or(0) == 0 {
            return;
        }
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let path = parts.next().unwrap_or_default().to_string();
        let mut length = 0;
        let mut authorization = None;
        loop {
            let mut line = String::new();
            if stream.read_line(&mut line).await.unwrap_or(0) == 0 {
                return;
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                let value = value.trim().to_string();
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.parse().unwrap(),
                    "authorization" => authorization = Some(value),
                    _ => {}
                }
            }
        }
        let mut body = vec![0; length];
        stream.read_exact(&mut body).await.unwrap();

        let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        max.fetch_max(now, Ordering::SeqCst);
        let index = count.fetch_add(1, Ordering::SeqCst);
        let reply = reply(index, &path);
        let _ = tx.send(Recorded {
            at: Instant::now(),
            method,
            path,
            authorization,
            body: String::from_utf8(body).unwrap(),
        });
        let answer = match reply {
            Reply::Hang => std::future::pending::<String>().await,
            Reply::After(hold) => {
                tokio::time::sleep(hold).await;
                "HTTP/1.1 204 No Content\r\n\r\n".to_string()
            }
            Reply::Status(status) => {
                format!("HTTP/1.1 {status}\r\ncontent-length: 4\r\n\r\nboom")
            }
        };
        // Out of flight before the answer is written, so the client cannot start its next
        // request while this one still counts.
        in_flight.fetch_sub(1, Ordering::SeqCst);
        if stream.get_mut().write_all(answer.as_bytes()).await.is_err() {
            return;
        }
    }
}

pub(super) fn answer_at_once(_: usize, _: &str) -> Reply {
    Reply::After(Duration::ZERO)
}

pub(super) fn event() -> EventId {
    EventId::from_partial("1889-B")
}

/// The shared client and key store, pointed at `site`, holding a key for `key_site`.
pub(super) fn shared(site: &str, key_site: &str) -> (SharedUwhPortalClient, SharedAccessKeys) {
    let client = UwhPortalClient::new(site, None, false, Duration::from_secs(10)).unwrap();
    let mut keys = SiteAccessKeys::default();
    keys.publish(
        key_site.to_string(),
        vec![AccessKey {
            site: key_site.to_string(),
            event: event(),
            key: KEY.to_string(),
        }],
    );
    (Arc::new(Mutex::new(client)), Arc::new(Mutex::new(keys)))
}
