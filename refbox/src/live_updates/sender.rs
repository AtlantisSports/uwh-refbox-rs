//! The background task that sends live updates to the portal.
//!
//! One loop, strictly sequential: each send is awaited before the next decision is made, which is
//! what keeps at most one send in flight. The frames arrive on a `watch` channel, which holds only
//! the newest one, so whatever changed while a send was in flight is coalesced into the next send
//! rather than queued behind it. Nothing here is retried or queued: a failed send is logged once
//! per distinct failure and dropped.
//!
//! This task never touches the score/stats upload queue or its health indicator.

use super::{LiveFrame, needs_send_now};
use crate::portal_manager::{SharedAccessKeys, SharedUwhPortalClient};
use log::{info, warn};
use std::{error::Error, future::Future, time::Duration};
use tokio::{sync::watch, time::Instant};

/// How often an unchanged frame is re-sent, and how long one send may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveTiming {
    pub heartbeat: Duration,
    pub request_timeout: Duration,
}

pub const LIVE_TIMING: LiveTiming = LiveTiming {
    heartbeat: Duration::from_secs(5),
    request_timeout: Duration::from_secs(3),
};

/// Start the live sender. It runs until `frames`' sender is dropped.
pub fn spawn_live_sender(
    frames: watch::Receiver<Option<LiveFrame>>,
    client: SharedUwhPortalClient,
    keys: SharedAccessKeys,
    timing: LiveTiming,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run(frames, client, keys, timing))
}

async fn run(
    mut frames: watch::Receiver<Option<LiveFrame>>,
    client: SharedUwhPortalClient,
    keys: SharedAccessKeys,
    timing: LiveTiming,
) {
    // The frame last attempted, and when that attempt started. Set together, after each attempt
    // completes -- success, failure or skip alike -- so a failure waits for a change or the next
    // heartbeat rather than being retried at once.
    let mut last_sent: Option<(LiveFrame, Instant)> = None;
    let mut log = FailureLog::default();

    loop {
        // Cloned out at once: the borrow holds the channel's lock and must not cross an await.
        let current = frames.borrow_and_update().clone();
        let Some(frame) = current else {
            last_sent = None;
            if frames.changed().await.is_err() {
                return;
            }
            continue;
        };

        let heartbeat_at = last_sent
            .as_ref()
            .filter(|_| frame.heartbeat)
            .map(|(_, at)| *at + timing.heartbeat);
        let due = needs_send_now(last_sent.as_ref().map(|(f, _)| f), &frame)
            || heartbeat_at.is_some_and(|at| at <= Instant::now());

        if due {
            let started = Instant::now();
            let outcome = match build_send(&client, &keys, &frame, timing.request_timeout) {
                Ok(send) => send.await.map_err(|why| classify(why.as_ref())),
                Err(skip) => Err(skip),
            };
            match log.record(outcome) {
                Some(LogLine::Warn(text)) => warn!("{text}"),
                Some(LogLine::Info(text)) => info!("{text}"),
                None => {}
            }
            last_sent = Some((frame, started));
            continue;
        }

        let changed = match heartbeat_at {
            Some(at) => tokio::select! {
                changed = frames.changed() => changed,
                () = tokio::time::sleep_until(at) => Ok(()),
            },
            None => frames.changed().await,
        };
        if changed.is_err() {
            return;
        }
    }
}

/// Build the send for `frame`, or say why it cannot be sent.
///
/// Both locks are taken keys first, then client -- the order `UwhPortalIo::request_for` and
/// `repoint_client` use, which keeps the pair free of deadlock and keeps a departing site's key
/// off the client now pointing at the new one. Both guards drop when this returns, before the
/// send is awaited.
fn build_send(
    client: &SharedUwhPortalClient,
    keys: &SharedAccessKeys,
    frame: &LiveFrame,
    timeout: Duration,
) -> Result<impl Future<Output = Result<(), Box<dyn Error>>> + use<>, Failure> {
    // why this cannot panic: `unwrap_or_else(into_inner)` keeps the task alive even if a writer
    // panicked while holding either lock.
    let keys = keys.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if keys.site() != frame.site {
        return Err(Failure::SiteChanged);
    }
    let key = keys.key_for(&frame.event).ok_or(Failure::NoKey)?;
    let client = client
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    client
        .put_live_game(key, &frame.event, &frame.game, &frame.state, timeout)
        .map_err(|why| Failure::Unbuildable(bounded(&why.to_string())))
}

/// Turn a send's error into an owned [`Failure`], so nothing borrowed or unsendable outlives the
/// await that produced it.
fn classify(why: &(dyn Error + 'static)) -> Failure {
    if let Some(transport) = why.downcast_ref::<reqwest::Error>() {
        if transport.is_timeout() {
            Failure::Timeout
        } else if transport.is_connect() {
            Failure::Connect
        } else {
            Failure::Transport(bounded(&transport.to_string()))
        }
    } else {
        // uwh-common's own error for a non-2xx reply reads "{status}: {body}".
        let text = why.to_string();
        let (status, body) = text.split_once(':').unwrap_or((text.as_str(), ""));
        Failure::Rejected {
            status: bounded(status.trim()),
            body: bounded(body.trim()),
        }
    }
}

/// The most of any reply or error text that is ever logged.
const MAX_LOGGED_CHARS: usize = 200;

fn bounded(text: &str) -> String {
    let mut chars = text.chars();
    let mut kept: String = chars.by_ref().take(MAX_LOGGED_CHARS).collect();
    if chars.next().is_some() {
        kept.push_str("...");
    }
    kept
}

/// Why a send did not get through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The published keys belong to a different site than the frame.
    SiteChanged,
    /// No access key is held for the frame's event on its site.
    NoKey,
    /// The request could not be built (for example, a key a header cannot carry).
    Unbuildable(String),
    /// The portal answered with a non-2xx status.
    Rejected {
        status: String,
        body: String,
    },
    Timeout,
    Connect,
    /// Any other transport failure.
    Transport(String),
}

/// What makes two failures "the same" for logging: the status, never the reply body, which may
/// differ on every request (trace ids) and would otherwise log every five seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FailureKind {
    SiteChanged,
    NoKey,
    Unbuildable,
    Status(String),
    Timeout,
    Connect,
    Transport,
}

impl Failure {
    fn kind(&self) -> FailureKind {
        match self {
            Self::SiteChanged => FailureKind::SiteChanged,
            Self::NoKey => FailureKind::NoKey,
            Self::Unbuildable(_) => FailureKind::Unbuildable,
            Self::Rejected { status, .. } => FailureKind::Status(status.clone()),
            Self::Timeout => FailureKind::Timeout,
            Self::Connect => FailureKind::Connect,
            Self::Transport(_) => FailureKind::Transport,
        }
    }

    /// What to log. Every piece of reply or error text in it is bounded.
    fn describe(&self) -> String {
        match self {
            Self::SiteChanged => {
                "not sent: the access keys belong to a different site than the game".to_string()
            }
            Self::NoKey => "not sent: no access key is held for this event".to_string(),
            Self::Unbuildable(why) => format!("the request could not be built: {}", bounded(why)),
            Self::Rejected { status, body } if body.is_empty() => {
                format!("the portal answered {}", bounded(status))
            }
            Self::Rejected { status, body } => {
                format!("the portal answered {}: {}", bounded(status), bounded(body))
            }
            Self::Timeout => "the portal did not answer in time".to_string(),
            Self::Connect => "could not connect to the portal".to_string(),
            Self::Transport(why) => format!("the request failed: {}", bounded(why)),
        }
    }
}

/// A line the failure log wants written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLine {
    Warn(String),
    Info(String),
}

/// Logs each distinct failure once, and the recovery once, instead of every heartbeat.
#[derive(Debug, Default)]
pub struct FailureLog {
    /// The kind of the failure last logged, while sends are failing.
    failing: Option<FailureKind>,
}

impl FailureLog {
    pub fn record(&mut self, outcome: Result<(), Failure>) -> Option<LogLine> {
        match outcome {
            Ok(()) => self.failing.take().map(|_| {
                LogLine::Info("live updates to the portal are getting through again".to_string())
            }),
            Err(failure) => {
                let kind = failure.kind();
                if self.failing.as_ref() == Some(&kind) {
                    return None;
                }
                self.failing = Some(kind);
                Some(LogLine::Warn(format!(
                    "live update to the portal failed, and is dropped: {}",
                    failure.describe()
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AccessKey;
    use crate::portal_manager::SiteAccessKeys;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        time::Instant,
    };
    use uwh_common::{
        game_snapshot::GamePeriod,
        uwhportal::{
            UwhPortalClient,
            live::{LiveGameState, LiveScores},
            schedule::EventId,
        },
    };

    const KEY: &str = "KEY-LIVE";

    /// How the fake portal answers the request with this index (0-based).
    #[derive(Clone, Copy)]
    enum Reply {
        /// Answer `204 No Content` after holding the request this long.
        After(Duration),
        /// Never answer.
        Hang,
    }

    struct Recorded {
        at: Instant,
        method: String,
        path: String,
        authorization: Option<String>,
        body: String,
    }

    struct Portal {
        site: String,
        requests: mpsc::UnboundedReceiver<Recorded>,
        max_in_flight: Arc<AtomicUsize>,
    }

    impl Portal {
        /// The next request, if one arrives within `within`.
        async fn next(&mut self, within: Duration) -> Option<Recorded> {
            tokio::time::timeout(within, self.requests.recv())
                .await
                .ok()
                .flatten()
        }

        /// Every request that arrives within `within`.
        async fn all_within(&mut self, within: Duration) -> Vec<Recorded> {
            let deadline = Instant::now() + within;
            let mut got = Vec::new();
            while let Ok(Some(r)) = tokio::time::timeout_at(deadline, self.requests.recv()).await {
                got.push(r);
            }
            got
        }
    }

    /// A fake portal on a loopback port. `reply` decides, per request, how it answers.
    async fn portal(reply: impl Fn(usize) -> Reply + Send + Sync + 'static) -> Portal {
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
        reply: Arc<impl Fn(usize) -> Reply>,
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
            let _ = tx.send(Recorded {
                at: Instant::now(),
                method,
                path,
                authorization,
                body: String::from_utf8(body).unwrap(),
            });
            match reply(index) {
                Reply::Hang => std::future::pending::<()>().await,
                Reply::After(hold) => tokio::time::sleep(hold).await,
            }
            // Out of flight before the answer is written, so the client cannot start its next
            // request while this one still counts.
            in_flight.fetch_sub(1, Ordering::SeqCst);
            if stream
                .get_mut()
                .write_all(b"HTTP/1.1 204 No Content\r\n\r\n")
                .await
                .is_err()
            {
                return;
            }
        }
    }

    fn answer_at_once(_: usize) -> Reply {
        Reply::After(Duration::ZERO)
    }

    fn event() -> EventId {
        EventId::from_partial("1889-B")
    }

    fn state(black: u8) -> LiveGameState {
        LiveGameState {
            period: GamePeriod::FirstHalf,
            clock_seconds: 300,
            clock_running: true,
            timeout: None,
            scores: Some(LiveScores { black, white: 1 }),
        }
    }

    fn frame(site: &str, black: u8) -> LiveFrame {
        LiveFrame {
            site: site.to_string(),
            event: event(),
            game: "5".to_string(),
            state: state(black),
            heartbeat: true,
        }
    }

    fn timing(heartbeat_ms: u64, timeout_ms: u64) -> LiveTiming {
        LiveTiming {
            heartbeat: Duration::from_millis(heartbeat_ms),
            request_timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// The shared client and key store, pointed at `site`, holding a key for `key_site`.
    fn shared(site: &str, key_site: &str) -> (SharedUwhPortalClient, SharedAccessKeys) {
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

    /// Start a sender against `portal` with `first` as its frame.
    fn start(
        portal: &Portal,
        first: Option<LiveFrame>,
        timing: LiveTiming,
    ) -> (
        watch::Sender<Option<LiveFrame>>,
        tokio::task::JoinHandle<()>,
    ) {
        let (client, keys) = shared(&portal.site, &portal.site);
        let (tx, rx) = watch::channel(first);
        (tx, spawn_live_sender(rx, client, keys, timing))
    }

    const WAIT: Duration = Duration::from_secs(2);

    #[tokio::test]
    async fn sends_put_with_body_and_key() {
        let mut portal = portal(answer_at_once).await;
        let (_tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(1000, 500));
        let got = portal.next(WAIT).await.expect("a request");
        assert_eq!(got.method, "PUT");
        assert_eq!(got.path, "/api/events/1889-B/live-games/5");
        assert_eq!(got.authorization.as_deref(), Some("Bearer KEY-LIVE"));
        assert_eq!(got.body, serde_json::to_string(&state(2)).unwrap());
    }

    #[tokio::test]
    async fn heartbeat_resends_unchanged_frame() {
        let mut portal = portal(answer_at_once).await;
        let (_tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(300, 150));
        let first = portal.next(WAIT).await.expect("a first request");
        let rest = portal.all_within(Duration::from_millis(750)).await;
        assert!(
            !rest.is_empty(),
            "an unchanged frame is re-sent every heartbeat"
        );
        assert_eq!(rest[0].body, first.body);
    }

    #[tokio::test]
    async fn change_sends_without_waiting_for_heartbeat() {
        let mut portal = portal(answer_at_once).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(2000, 500));
        portal.next(WAIT).await.expect("a first request");
        let changed = Instant::now();
        tx.send(Some(frame(&portal.site, 3))).unwrap();
        let second = portal.next(WAIT).await.expect("a second request");
        assert!(
            second.at - changed < Duration::from_millis(500),
            "sent {:?} after the change; the heartbeat is 2 s",
            second.at - changed
        );
        assert_eq!(second.body, serde_json::to_string(&state(3)).unwrap());
    }

    #[tokio::test]
    async fn stalled_server_is_abandoned_and_next_send_proceeds() {
        let mut portal = portal(|i| {
            if i == 0 {
                Reply::Hang
            } else {
                Reply::After(Duration::ZERO)
            }
        })
        .await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(2000, 200));
        let first = portal.next(WAIT).await.expect("a first request");
        // A change while the first is stuck: it waits for the first to be abandoned, then goes.
        tx.send(Some(frame(&portal.site, 3))).unwrap();
        let second = portal.next(WAIT).await.expect("a second request");
        let gap = second.at - first.at;
        assert!(
            gap >= Duration::from_millis(150) && gap < Duration::from_millis(1000),
            "second request {gap:?} after the first; the timeout is 200 ms"
        );
        assert_eq!(second.body, serde_json::to_string(&state(3)).unwrap());
    }

    #[tokio::test]
    async fn never_two_in_flight() {
        let mut portal = portal(|_| Reply::After(Duration::from_millis(600))).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 0)), timing(300, 2000));
        portal.next(WAIT).await.expect("a first request");
        // Changes (all within the first 600 ms hold) and overdue heartbeats pile up while each
        // request is held.
        for black in 1..=4 {
            tokio::time::sleep(Duration::from_millis(80)).await;
            tx.send(Some(frame(&portal.site, black))).unwrap();
        }
        let rest = portal.all_within(Duration::from_millis(1500)).await;
        assert!(!rest.is_empty(), "sending went on");
        assert_eq!(portal.max_in_flight.load(Ordering::SeqCst), 1);
        // Coalesced: the changes made during the first send go out as one, the newest.
        assert_eq!(rest[0].body, serde_json::to_string(&state(4)).unwrap());
    }

    #[tokio::test]
    async fn finished_court_sends_once() {
        let mut portal = portal(answer_at_once).await;
        let finished = LiveFrame {
            heartbeat: false,
            ..frame(&portal.site, 2)
        };
        let (_tx, _task) = start(&portal, Some(finished), timing(300, 150));
        let got = portal.all_within(Duration::from_millis(1000)).await;
        assert_eq!(got.len(), 1);
    }

    #[tokio::test]
    async fn none_stops_sending() {
        let mut portal = portal(answer_at_once).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(300, 150));
        portal.next(WAIT).await.expect("a first request");
        tx.send(None).unwrap();
        let got = portal.all_within(Duration::from_millis(1000)).await;
        assert_eq!(got.len(), 0, "nothing is sent while there is no frame");
        // What was last sent is forgotten, so the same frame goes out again at once.
        tx.send(Some(frame(&portal.site, 2))).unwrap();
        assert!(portal.next(Duration::from_millis(250)).await.is_some());
    }

    #[tokio::test]
    async fn skips_when_site_changed() {
        let mut portal = portal(answer_at_once).await;
        let (client, keys) = shared(&portal.site, "https://other.example");
        let (_tx, rx) = watch::channel(Some(frame(&portal.site, 2)));
        let _task = spawn_live_sender(rx, client, keys, timing(300, 150));
        assert_eq!(
            portal.all_within(Duration::from_millis(1000)).await.len(),
            0
        );
    }

    #[tokio::test]
    async fn skips_without_key() {
        let mut portal = portal(answer_at_once).await;
        let (client, keys) = shared(&portal.site, &portal.site);
        keys.lock()
            .unwrap()
            .publish(portal.site.clone(), Vec::new());
        let (_tx, rx) = watch::channel(Some(frame(&portal.site, 2)));
        let _task = spawn_live_sender(rx, client, keys, timing(300, 150));
        assert_eq!(
            portal.all_within(Duration::from_millis(1000)).await.len(),
            0
        );
    }

    #[tokio::test]
    async fn ends_when_the_channel_closes() {
        let portal = portal(answer_at_once).await;
        let (tx, task) = start(&portal, None, timing(300, 150));
        drop(tx);
        tokio::time::timeout(WAIT, task)
            .await
            .expect("the task ends")
            .unwrap();
    }

    #[test]
    fn failure_log_reports_each_distinct_failure_once() {
        let rejected = |body: &str| Failure::Rejected {
            status: "500 Internal Server Error".to_string(),
            body: body.to_string(),
        };
        let mut log = FailureLog::default();
        let lines: Vec<_> = (0..10)
            .filter_map(|i| log.record(Err(rejected(&format!("trace {i}")))))
            .collect();
        assert_eq!(lines.len(), 1, "same status, varying body: one line");
        assert!(matches!(lines[0], LogLine::Warn(_)));

        assert!(matches!(
            log.record(Err(Failure::Timeout)),
            Some(LogLine::Warn(_))
        ));
        assert_eq!(log.record(Err(Failure::Timeout)), None);
        assert!(matches!(
            log.record(Err(Failure::NoKey)),
            Some(LogLine::Warn(_))
        ));
        assert!(matches!(log.record(Ok(())), Some(LogLine::Info(_))));
        assert_eq!(log.record(Ok(())), None);
    }

    #[test]
    fn first_success_logs_nothing_and_long_bodies_are_bounded() {
        let mut log = FailureLog::default();
        assert_eq!(log.record(Ok(())), None);
        let line = log.record(Err(Failure::Rejected {
            status: "400 Bad Request".to_string(),
            body: "x".repeat(5000),
        }));
        let Some(LogLine::Warn(text)) = line else {
            panic!("expected a warning, got {line:?}");
        };
        assert!(text.contains("400 Bad Request"));
        assert!(text.len() < 400, "{} characters logged", text.len());
    }
}
