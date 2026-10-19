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
use log::{debug, info, warn};
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
    // heartbeat rather than being retried at once, and a skip does not spin the loop.
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
            match log.record(outcome, Instant::now()) {
                Some(LogLine::Warn(text)) => warn!("{text}"),
                Some(LogLine::Info(text)) => info!("{text}"),
                Some(LogLine::Debug(text)) => debug!("{text}"),
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
                "the access keys belong to a different site than the game".to_string()
            }
            Self::NoKey => "no access key is held for this event".to_string(),
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
    /// A send condition not met (no key, or the site changed): not a failure.
    Debug(String),
}

/// The shortest gap between two log lines about the same kind of failure, or between two
/// recovery lines. A link that flaps all day logs a few lines an hour, not one every heartbeat.
pub const LOG_QUIET_WINDOW: Duration = Duration::from_secs(600);

/// Logs each kind of failure the first time it is seen and then at most once per
/// [`LOG_QUIET_WINDOW`], and the recovery at most once per window, instead of every heartbeat.
#[derive(Debug, Default)]
pub struct FailureLog {
    /// When each kind of failure was last logged.
    logged: Vec<(FailureKind, Instant)>,
    /// Sends that failed since the last recovery line.
    dropped: u32,
    /// When the last recovery line was logged.
    last_recovery: Option<Instant>,
}

impl FailureLog {
    pub fn record(&mut self, outcome: Result<(), Failure>, now: Instant) -> Option<LogLine> {
        let quiet = |since: Instant| now.saturating_duration_since(since) < LOG_QUIET_WINDOW;
        match outcome {
            Ok(()) => {
                if self.dropped == 0 || self.last_recovery.is_some_and(quiet) {
                    return None;
                }
                let dropped = std::mem::take(&mut self.dropped);
                self.last_recovery = Some(now);
                Some(LogLine::Info(format!(
                    "live updates to the portal are getting through again \
                     ({dropped} live updates dropped)"
                )))
            }
            // A send condition not met is not a failure: it is neither warned about nor counted
            // as dropped.
            Err(skip @ (Failure::SiteChanged | Failure::NoKey)) => Some(LogLine::Debug(format!(
                "live update to the portal not sent: {}",
                skip.describe()
            ))),
            Err(failure) => {
                self.dropped = self.dropped.saturating_add(1);
                let kind = failure.kind();
                match self.logged.iter_mut().find(|(k, _)| *k == kind) {
                    Some((_, at)) if quiet(*at) => return None,
                    Some((_, at)) => *at = now,
                    None => self.logged.push((kind, now)),
                }
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
        bundles::BlackWhiteBundle,
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
        /// Answer at once with this status line and a short body.
        Status(&'static str),
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

    /// A fake portal on a loopback port. `reply` decides, per request (its index and path), how it
    /// answers.
    async fn portal(reply: impl Fn(usize, &str) -> Reply + Send + Sync + 'static) -> Portal {
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

    fn answer_at_once(_: usize, _: &str) -> Reply {
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
            last_goal: None,
            penalties: None,
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

    /// How long to wait for a request that is expected. Generous on purpose: CI runners can stall
    /// for hundreds of milliseconds, and a request that is merely slow should not fail a test.
    /// Every timing bound below keeps at least ~500 ms between the expected value and the
    /// assertion, for the same reason.
    const WAIT: Duration = Duration::from_secs(5);

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
        let (_tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(2000, 500));
        let first = portal.next(WAIT).await.expect("a first request");
        let rest = portal.all_within(Duration::from_millis(3000)).await;
        assert!(
            !rest.is_empty(),
            "an unchanged frame is re-sent every heartbeat"
        );
        assert_eq!(rest[0].body, first.body);
        // The heartbeat sets the data cost. The first resend is due 2 s after the first send; a
        // stall can only bring the gap down by the time the first request took to arrive, so
        // 1 s leaves room for that while still catching a resend made without waiting.
        let gap = rest[0].at - first.at;
        assert!(
            gap >= Duration::from_millis(1000),
            "resent {gap:?} after the first; the heartbeat is 2 s"
        );
        // A 3 s window holds one resend (at 2 s; the next is due at 4 s), so more than 2 means it
        // is resending faster than the heartbeat.
        assert!(
            rest.len() <= 2,
            "{} resends in 3 s; the heartbeat is 2 s",
            rest.len()
        );
    }

    #[tokio::test]
    async fn change_sends_without_waiting_for_heartbeat() {
        let mut portal = portal(answer_at_once).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(3000, 500));
        portal.next(WAIT).await.expect("a first request");
        let changed = Instant::now();
        tx.send(Some(frame(&portal.site, 3))).unwrap();
        let second = portal.next(WAIT).await.expect("a second request");
        // Expected almost at once; waiting for the heartbeat would take about 3 s.
        assert!(
            second.at - changed < Duration::from_millis(1000),
            "sent {:?} after the change; the heartbeat is 3 s",
            second.at - changed
        );
        assert_eq!(second.body, serde_json::to_string(&state(3)).unwrap());
    }

    #[tokio::test]
    async fn stalled_server_is_abandoned_and_next_send_proceeds() {
        let mut portal = portal(|i, _| {
            if i == 0 {
                Reply::Hang
            } else {
                Reply::After(Duration::ZERO)
            }
        })
        .await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(4000, 1000));
        let first = portal.next(WAIT).await.expect("a first request");
        // A change while the first is stuck: it waits for the first to be abandoned, then goes.
        tx.send(Some(frame(&portal.site, 3))).unwrap();
        let second = portal.next(WAIT).await.expect("a second request");
        // Expected about 1 s (the timeout). Not waiting for the first gives ~0; ignoring the
        // timeout leaves it to the client's own 10 s one.
        let gap = second.at - first.at;
        assert!(
            gap >= Duration::from_millis(500) && gap < Duration::from_millis(3000),
            "second request {gap:?} after the first; the timeout is 1 s"
        );
        assert_eq!(second.body, serde_json::to_string(&state(3)).unwrap());
    }

    #[tokio::test]
    async fn never_two_in_flight() {
        let mut portal = portal(|_, _| Reply::After(Duration::from_millis(1500))).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 0)), timing(300, 4000));
        portal.next(WAIT).await.expect("a first request");
        // Changes (all within the first 1.5 s hold, which they need only ~320 ms of) and overdue
        // heartbeats pile up while each request is held.
        for black in 1..=4 {
            tokio::time::sleep(Duration::from_millis(80)).await;
            tx.send(Some(frame(&portal.site, black))).unwrap();
        }
        let rest = portal.all_within(Duration::from_millis(3000)).await;
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
        // A generous request timeout, so a slow runner cannot abandon the one send before it lands.
        let (_tx, _task) = start(&portal, Some(finished), timing(300, 1500));
        let got = portal.all_within(Duration::from_millis(1000)).await;
        assert_eq!(got.len(), 1);
    }

    #[tokio::test]
    async fn none_stops_sending() {
        let mut portal = portal(answer_at_once).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(3000, 500));
        portal.next(WAIT).await.expect("a first request");
        tx.send(None).unwrap();
        let got = portal.all_within(Duration::from_millis(1000)).await;
        assert_eq!(got.len(), 0, "nothing is sent while there is no frame");
        // What was last sent is forgotten, so the same frame goes out again at once -- not at the
        // next heartbeat, which is still ~2 s away.
        tx.send(Some(frame(&portal.site, 2))).unwrap();
        assert!(portal.next(Duration::from_millis(1000)).await.is_some());
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
        let now = Instant::now();
        let rejected = |body: &str| Failure::Rejected {
            status: "500 Internal Server Error".to_string(),
            body: body.to_string(),
        };
        let mut log = FailureLog::default();
        let lines: Vec<_> = (0..10)
            .filter_map(|i| log.record(Err(rejected(&format!("trace {i}"))), now))
            .collect();
        assert_eq!(lines.len(), 1, "same status, varying body: one line");
        assert!(matches!(lines[0], LogLine::Warn(_)));

        assert!(matches!(
            log.record(Err(Failure::Timeout), now),
            Some(LogLine::Warn(_))
        ));
        assert_eq!(log.record(Err(Failure::Timeout), now), None);
        let Some(LogLine::Info(text)) = log.record(Ok(()), now) else {
            panic!("expected the recovery line");
        };
        assert!(text.contains("(12 live updates dropped)"), "{text}");
        assert_eq!(log.record(Ok(()), now), None);
    }

    #[test]
    fn first_success_logs_nothing_and_long_bodies_are_bounded() {
        let now = Instant::now();
        let mut log = FailureLog::default();
        assert_eq!(log.record(Ok(()), now), None);
        let line = log.record(
            Err(Failure::Rejected {
                status: "400 Bad Request".to_string(),
                body: "x".repeat(5000),
            }),
            now,
        );
        let Some(LogLine::Warn(text)) = line else {
            panic!("expected a warning, got {line:?}");
        };
        assert!(text.contains("400 Bad Request"));
        assert!(text.len() < 400, "{} characters logged", text.len());
    }

    /// A link flapping between two kinds of failure, one attempt every 5 s.
    #[test]
    fn flapping_kinds_log_each_kind_once_per_window() {
        let start = Instant::now();
        let mut log = FailureLog::default();
        let warns = (0..20)
            .filter_map(|i| {
                let failure = if i % 2 == 0 {
                    Failure::Timeout
                } else {
                    Failure::Connect
                };
                log.record(Err(failure), start + Duration::from_secs(5 * i))
            })
            .inspect(|line| assert!(matches!(line, LogLine::Warn(_)), "{line:?}"))
            .count();
        assert_eq!(warns, 2);
    }

    /// Sends alternately failing and succeeding, one attempt every 5 s.
    #[test]
    fn flapping_fail_succeed_logs_once_per_window() {
        let start = Instant::now();
        let mut log = FailureLog::default();
        let lines: Vec<_> = (0..20)
            .filter_map(|i| {
                let outcome = if i % 2 == 0 {
                    Err(Failure::Timeout)
                } else {
                    Ok(())
                };
                log.record(outcome, start + Duration::from_secs(5 * i))
            })
            .collect();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(matches!(lines[0], LogLine::Warn(_)));
        let LogLine::Info(text) = &lines[1] else {
            panic!("expected the recovery line, got {lines:?}");
        };
        assert!(text.contains("(1 live updates dropped)"), "{text}");
    }

    #[test]
    fn kind_relogged_after_window() {
        let start = Instant::now();
        let mut log = FailureLog::default();
        assert!(log.record(Err(Failure::Timeout), start).is_some());
        assert_eq!(
            log.record(Err(Failure::Timeout), start + Duration::from_secs(599)),
            None
        );
        assert!(matches!(
            log.record(Err(Failure::Timeout), start + Duration::from_secs(601)),
            Some(LogLine::Warn(_))
        ));
    }

    #[test]
    fn classify_reads_status_from_rejection() {
        let why: Box<dyn Error> = "500 Internal Server Error: boom".into();
        assert_eq!(
            classify(why.as_ref()),
            Failure::Rejected {
                status: "500 Internal Server Error".to_string(),
                body: "boom".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn rejected_send_waits_for_heartbeat() {
        let mut portal = portal(|_, _| Reply::Status("500 Internal Server Error")).await;
        let (_tx, _task) = start(&portal, Some(frame(&portal.site, 2)), timing(2000, 500));
        let first = portal.next(WAIT).await.expect("a first request");
        let second = portal.next(WAIT).await.expect("a second request");
        // Expected about 2 s; an immediate retry gives ~0. A stall can only bring the gap down by
        // the time the first request took to arrive, which 1 s leaves room for.
        let gap = second.at - first.at;
        assert!(
            gap >= Duration::from_millis(1000),
            "a rejected send was retried {gap:?} later; the heartbeat is 2 s"
        );
    }

    /// A missing key, or keys for another site, is a send condition not met: logged at debug
    /// every time, never warned about, and never counted as a dropped update.
    #[test]
    fn missing_key_is_not_logged_as_a_failure() {
        let now = Instant::now();
        let mut log = FailureLog::default();
        for skip in [Failure::NoKey, Failure::SiteChanged, Failure::NoKey] {
            let line = log.record(Err(skip), now);
            assert!(matches!(line, Some(LogLine::Debug(_))), "{line:?}");
        }
        assert_eq!(log.record(Ok(()), now), None, "nothing was dropped");

        assert!(matches!(
            log.record(Err(Failure::Timeout), now),
            Some(LogLine::Warn(_))
        ));
        assert!(matches!(
            log.record(Err(Failure::NoKey), now),
            Some(LogLine::Debug(_))
        ));
        let Some(LogLine::Info(text)) = log.record(Ok(()), now) else {
            panic!("expected the recovery line");
        };
        assert!(text.contains("(1 live updates dropped)"), "{text}");
    }

    /// Anti-goal #1: a live send stuck on the portal must not hold up a score post made through
    /// the same shared client. Multi-threaded, and the post's lock taken on a blocking thread, so
    /// that a sender holding the client lock across its send shows up as a timeout here rather
    /// than as a hung test.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stalled_live_send_does_not_delay_score_post() {
        let mut portal = portal(|_, path| {
            if path.contains("/live-games/") {
                Reply::Hang
            } else {
                // `post_game_scores` accepts exactly 200.
                Reply::Status("200 OK")
            }
        })
        .await;
        let (client, keys) = shared(&portal.site, &portal.site);
        let (_tx, rx) = watch::channel(Some(frame(&portal.site, 2)));
        let _task = spawn_live_sender(rx, Arc::clone(&client), keys, timing(20_000, 10_000));
        let live = portal.next(WAIT).await.expect("the live PUT");
        assert!(live.path.contains("/live-games/"), "{}", live.path);

        // The live PUT is now hung, and will be for 10 s. The post is expected in milliseconds;
        // 3 s leaves room for a slow runner while still well short of the hung send's 10 s.
        let started = Instant::now();
        let posting = tokio::task::spawn_blocking(move || {
            let guard = client
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.post_game_scores(
                &event(),
                &"5".to_string(),
                BlackWhiteBundle { black: 2, white: 1 },
                false,
            )
        });
        let posted = tokio::time::timeout(Duration::from_secs(3), async {
            posting.await.expect("the lock is taken").await
        })
        .await;
        let elapsed = started.elapsed();
        assert!(
            matches!(posted, Ok(Ok(()))),
            "the score post did not complete within 3 s of a hung live send: {:?}",
            posted.map(|r| r.map_err(|e| e.to_string()))
        );
        assert!(elapsed < Duration::from_secs(3), "{elapsed:?}");
        let post = portal.next(WAIT).await.expect("the score post");
        assert!(post.path.contains("/scores"), "{}", post.path);
    }
}
