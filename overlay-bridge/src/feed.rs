//! Reads a refbox's network feed: one JSON-encoded `GameSnapshot` per line, terminated by `\n`.
//!
//! The refbox does not frame messages by length, only by newline, and a single message can be
//! larger than any one `read()` call returns. This reader buffers bytes until a full line is
//! available, so a message's size never determines whether it can be parsed. This matters because
//! the existing `overlay` crate reads into a fixed 1024-byte buffer without looking for the
//! newline: once a message exceeds that buffer, its reads desynchronise and every subsequent
//! message fails to parse. Nothing here bounds message size.
//!
//! [`Supervisor::run`], further down, owns the connection itself: it (re)connects, configures TCP
//! keepalive so a refbox that silently disappears is noticed instead of hanging a read forever
//! (see its doc for why), and reconnects on any kind of loss. `SnapshotReader` above stays exactly
//! what it was -- unbounded and connection-agnostic -- because bounding a *connection*, as opposed
//! to a message, is the supervisor's business, not the reader's.
//!
//! [`Connection`] and [`ConnectionState`], further down still, are what the supervisor publishes
//! its connection status through. This is now load-bearing rather than incidental (spec §4.6,
//! §5.4): the bridge no longer projects the clock forward through a dropout, so whether a served
//! table shows real values or blanks them is decided by nothing but the connection's own
//! liveness -- never by how long it has been since a message arrived, because a stopped clock
//! produces exactly that kind of silence legitimately. [`ConnectionState`] also carries whether
//! keepalive is actually configured right now (Task 7, [`ConnectionState::keepalive_active`]) --
//! the supervisor logs a configuration failure to stderr and keeps reading regardless (see
//! `configure_keepalive`'s doc for why), but stderr is invisible to an operator running a
//! compiled program, so this is the structured signal the status page surfaces instead.

use std::{
    fmt, io,
    pin::Pin,
    sync::{
        Arc, PoisonError, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{Stream, StreamExt};
use socket2::{SockRef, TcpKeepalive};
use tokio::{
    io::{AsyncRead, ReadBuf},
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc,
    time::sleep,
};
use uwh_common::game_snapshot::GameSnapshot;

/// How many bytes to ask the underlying reader for per read call. This is purely an I/O
/// scheduling granularity, not a limit on message size: a message larger than this many bytes is
/// simply assembled from more than one read into the unbounded `line_buf` below.
const READ_CHUNK_BYTES: usize = 4096;

/// Reads a refbox's newline-framed `GameSnapshot` feed and yields one snapshot per line.
///
/// Bytes are accumulated in an unbounded buffer and split on `b'\n'`, so a message is parsed
/// correctly no matter how the underlying reads happen to be chunked, or how large it is.
pub struct SnapshotReader<R> {
    reader: R,
    line_buf: Vec<u8>,
    eof: bool,
}

impl<R> SnapshotReader<R>
where
    R: AsyncRead + Unpin,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: Vec::new(),
            eof: false,
        }
    }

    /// If `line_buf` holds a complete newline-terminated line, remove and return it, without the
    /// trailing newline. Leaves `line_buf` untouched if no full line is buffered yet.
    fn take_line(&mut self) -> Option<Vec<u8>> {
        let newline_pos = self.line_buf.iter().position(|&b| b == b'\n')?;
        let mut line: Vec<u8> = self.line_buf.drain(..=newline_pos).collect();
        line.pop(); // drop the trailing '\n'
        Some(line)
    }
}

impl<R> Stream for SnapshotReader<R>
where
    R: AsyncRead + Unpin,
{
    type Item = Result<GameSnapshot, FeedError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(line) = this.take_line() {
                return Poll::Ready(Some(parse_line(&line)));
            }

            if this.eof {
                // Any unterminated bytes left in line_buf were never completed by a newline
                // before the connection ended; there's no way to know whether they were a real
                // message cut short or nothing at all, so they are dropped rather than reported.
                return Poll::Ready(None);
            }

            let mut scratch = [0u8; READ_CHUNK_BYTES];
            let read = {
                let mut read_buf = ReadBuf::new(&mut scratch);
                match Pin::new(&mut this.reader).poll_read(cx, &mut read_buf) {
                    Poll::Ready(Ok(())) => read_buf.filled().len(),
                    Poll::Ready(Err(e)) => {
                        this.eof = true;
                        return Poll::Ready(Some(Err(FeedError::Io(e))));
                    }
                    Poll::Pending => return Poll::Pending,
                }
            };

            if read == 0 {
                this.eof = true;
            } else {
                this.line_buf.extend_from_slice(&scratch[..read]);
            }
        }
    }
}

fn parse_line(line: &[u8]) -> Result<GameSnapshot, FeedError> {
    serde_json::from_slice(line).map_err(FeedError::Parse)
}

/// Errors that can occur while reading `GameSnapshot`s from a refbox's network feed.
#[derive(Debug)]
pub enum FeedError {
    /// The underlying connection could not be read from.
    Io(io::Error),
    /// A line from the feed was not a valid `GameSnapshot`.
    Parse(serde_json::Error),
}

impl fmt::Display for FeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeedError::Io(e) => write!(f, "error reading from refbox feed: {e}"),
            FeedError::Parse(e) => write!(f, "could not parse a snapshot from the feed: {e}"),
        }
    }
}

impl std::error::Error for FeedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FeedError::Io(e) => Some(e),
            FeedError::Parse(e) => Some(e),
        }
    }
}

/// How many consecutive bytes a peer may send with no newline before the supervisor treats it as
/// misbehaving and drops the connection, rather than letting `SnapshotReader`'s unbounded internal
/// buffer grow forever.
///
/// This is not a message-size limit -- `SnapshotReader` deliberately has none (see the module doc)
/// -- it guards against a peer that is not sending a refbox feed at all. The bridge only ever
/// connects *out*, to an address the operator configured, so the realistic way to trigger this is
/// `--refbox-port` pointed at the wrong service (a web server, a database, ...) that keeps the
/// connection open and keeps writing bytes with no newline in them; without a cap, that would grow
/// memory without bound. The cap sits two orders of magnitude above the largest message exercised
/// anywhere in this file's own tests (a synthetic message just over 8 KB), so no real snapshot,
/// however unusually large, can come close to tripping it.
const MAX_UNTERMINATED_BYTES: usize = 1024 * 1024;

/// Wraps an `AsyncRead`, counting bytes seen since the last `b'\n'`, and fails the read with an
/// `io::Error` once that count would exceed [`MAX_UNTERMINATED_BYTES`].
///
/// This lives here, separate from `SnapshotReader`, on purpose: bounding message size is not
/// `SnapshotReader`'s job (see the module doc), but guarding against a misbehaving *connection* is
/// the supervisor's job, so the guard sits in front of the reader instead of inside it. The
/// `io::Error` this produces surfaces through `SnapshotReader` as `FeedError::Io`, so it is picked
/// up by the supervisor's existing "connection lost -> reconnect" handling with no special-casing.
struct LineLimited<R> {
    reader: R,
    /// Bytes seen since the last `b'\n'` (or since the connection opened).
    unterminated: usize,
}

impl<R> LineLimited<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            unterminated: 0,
        }
    }
}

impl<R> AsyncRead for LineLimited<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let result = Pin::new(&mut this.reader).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            let newly_read = &buf.filled()[before..];
            match newly_read.iter().rposition(|&b| b == b'\n') {
                Some(pos) => this.unterminated = newly_read.len() - pos - 1,
                None => this.unterminated += newly_read.len(),
            }
            if this.unterminated > MAX_UNTERMINATED_BYTES {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "peer sent over {MAX_UNTERMINATED_BYTES} bytes with no newline; \
                         dropping the connection"
                    ),
                )));
            }
        }
        result
    }
}

/// How long a socket may sit idle before the OS starts sending TCP keepalive probes.
const KEEPALIVE_IDLE: Duration = Duration::from_secs(5);
/// How far apart consecutive keepalive probes are sent once idle.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(3);
/// How many unanswered probes the OS sends before giving up on the connection.
const KEEPALIVE_RETRIES: u32 = 3;

/// Configures TCP keepalive on `stream` so a refbox that silently disappears -- Wi-Fi drop, power
/// cycle -- is noticed by the OS and reported as a read error, instead of the read waiting
/// forever. This is the fix for the bug described in the module doc: the feed is one-way, so the
/// bridge never transmits, and a peer that has rebooted never gets the chance to reset the
/// connection, because nothing is ever sent to it for it to reset.
///
/// With the settings above, a dead peer is noticed within `KEEPALIVE_IDLE + KEEPALIVE_RETRIES *
/// KEEPALIVE_INTERVAL` = 5s + 3 * 3s = 14s in the worst case -- inside the "roughly ten to fifteen
/// seconds" the design calls for, and comfortably short of the 25-second stopped-clock silence
/// measured on a live refbox on 2026-08-26, which is exactly why a read timeout was rejected in
/// favour of this instead (see the module doc).
fn configure_keepalive(stream: &TcpStream) -> io::Result<()> {
    let params = TcpKeepalive::new()
        .with_time(KEEPALIVE_IDLE)
        .with_interval(KEEPALIVE_INTERVAL)
        .with_retries(KEEPALIVE_RETRIES);
    SockRef::from(stream).set_tcp_keepalive(&params)
}

/// How long to wait before a (re)connect attempt, whether the previous one was refused or an
/// established connection was just lost.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Whether the bridge's feed connection to the refbox is alive right now -- and, if not, whether
/// it ever has been. Judged **entirely by the connection itself**: a successful connect, a read
/// error, an end-of-stream, or a keepalive probe reporting the peer gone. **Never** by how long it
/// has been since a message arrived -- see the module doc: the refbox goes completely silent
/// whenever the clock is stopped (25 seconds observed), so silence can never be evidence the
/// connection is down, or the graphic would vanish every time the referee stops the clock.
///
/// `NeverConnected` and `Disconnected` are kept as distinct variants, not collapsed into a single
/// "not connected" state, so a caller can tell "hasn't found a refbox yet" apart from "found one,
/// then lost it" -- a bridge that has never once connected has nothing meaningful to say about
/// when it was last in contact, which a bridge that just lost a connection does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connection {
    /// No connection has ever been established since the bridge started.
    NeverConnected,
    /// The connection is live right now.
    Connected,
    /// A connection was established at least once, and has since been lost.
    Disconnected,
}

impl Connection {
    /// Whether a served table should show the refbox's real values right now. Only true while
    /// the connection is actually live -- both `NeverConnected` and `Disconnected` mean "nothing
    /// trustworthy to show" (see `tables`' module doc for what happens to a table's values when
    /// this is false).
    pub fn is_live(self) -> bool {
        matches!(self, Connection::Connected)
    }
}

/// One consistent read of both [`Connection`] and, when disconnected, how long it has been --
/// returned together, from a single lock acquisition, specifically so the two can never be
/// observed to disagree.
///
/// **This exists because they used to be able to disagree.** Task 7's first version tracked the
/// drop duration in a *separate* background-polled watcher, reading `ConnectionState::get()` on
/// its own timer independently of whatever reads `ConnectionState` for the connection flag
/// itself. A caller (`server.rs`) that read the connection flag and the duration as two separate
/// calls could observe them from two different moments: on reconnect, a window of up to the
/// watcher's poll interval existed where `get()` already read `Connected` but the watcher had not
/// yet noticed and cleared its own stale duration -- a served page showing a green "Connected"
/// indicator above a red "Down for 42s" line. Review caught it (Task 7 fix round 1). Gating the
/// duration's *display* on the connection flag at the point of use would only have hidden the
/// symptom in the HTML page while leaving `/status.json`'s raw `disconnectedForSeconds` field
/// just as capable of disagreeing with `contact` -- the two pieces would still come from two
/// independently-timed sources, still capable of disagreeing, just less visibly so. The actual
/// fix is this type: `disconnected_for` is now recorded *at the exact moment*
/// [`ConnectionState::set_disconnected`] is called, in the same lock acquisition that changes
/// `Connection` itself, and read back through [`ConnectionState::snapshot`] in one more single
/// acquisition -- so a caller can no longer construct a comparison between two different
/// moments in time, because there is only ever one read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionStatus {
    pub connection: Connection,
    /// How long the connection has been continuously disconnected, or `None` if it is not
    /// currently disconnected -- this covers both `Connected` and `NeverConnected` alike, because
    /// `NeverConnected` has no drop instant to measure from (see [`Connection`]'s doc).
    pub disconnected_for: Option<Duration>,
}

/// [`ConnectionState`]'s internal storage: [`Connection`] and, when it is `Disconnected`, exactly
/// when that happened -- updated together, under one lock, by [`ConnectionState::set_connected`]
/// and [`ConnectionState::set_disconnected`]. See [`ConnectionStatus`]'s doc for why this is one
/// struct behind one lock rather than two independently-updated pieces of state.
#[derive(Debug, Clone, Copy)]
struct ConnectionInner {
    connection: Connection,
    disconnected_at: Option<Instant>,
}

/// A cheaply cloneable handle to the bridge's live [`Connection`] state (plus, since Task 7, when
/// it last dropped), shared between [`Supervisor::run`] (the only thing that ever writes it) and
/// the HTTP server (which reads it, via [`ConnectionState::get`]/[`ConnectionState::snapshot`], to
/// decide whether to serve real values or blank ones, and how long it's been down). Backed by a
/// lock rather than a bare atomic: `Connection` alone was a small `Copy` value with no multi-step
/// update to make atomic, but pairing it with *when* it changed (see [`ConnectionStatus`]) is
/// exactly a multi-step update that must be seen as a whole, so a lock is what actually makes
/// that true rather than merely convenient.
///
/// Also carries whether TCP keepalive is actually configured right now (Task 7) -- a separate,
/// independent atomic, not folded into [`ConnectionInner`], because it answers a different
/// question ("is the mechanism that detects a dead refbox running at all") from the one
/// `Connection` answers ("is the refbox reachable right now"), and nothing ever needs to read it
/// paired atomically with the other two -- see [`ConnectionState::keepalive_active`]'s doc for why
/// an operator needs to see it at all.
#[derive(Debug, Clone)]
pub struct ConnectionState {
    inner: Arc<RwLock<ConnectionInner>>,
    keepalive_active: Arc<AtomicBool>,
}

impl ConnectionState {
    /// A fresh handle reporting [`Connection::NeverConnected`] (with no drop instant -- there is
    /// nothing to measure from) and `keepalive_active() == true`, for the bridge's startup state
    /// before any connection attempt has been made. Starting keepalive `true` (rather than some
    /// third "unknown" state) matches this crate's no-chicken-and-egg principle for the status
    /// page: `configure_keepalive` fails to apply only in rare platform/network edge cases (see
    /// its own doc), so assuming success is more useful to show before the first attempt than an
    /// "unknown" a real operator would have to interpret.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ConnectionInner {
                connection: Connection::NeverConnected,
                disconnected_at: None,
            })),
            keepalive_active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// The connection state right now. Equivalent to `self.snapshot().connection`, kept as its
    /// own method because most callers (every table handler, most of this crate's own tests) only
    /// ever need this half of [`ConnectionStatus`] and have no duration to keep consistent with
    /// it.
    pub fn get(&self) -> Connection {
        self.snapshot().connection
    }

    /// [`Connection`] and, when disconnected, how long it has been -- read together from a single
    /// lock acquisition. Use this (never `get()` plus a separately-tracked duration) anywhere both
    /// pieces are served or displayed together, such as `/status.json` and the operator status
    /// page -- see [`ConnectionStatus`]'s doc for why that distinction is the actual fix for a
    /// real bug this crate shipped once already.
    pub fn snapshot(&self) -> ConnectionStatus {
        let inner = self.inner.read().unwrap_or_else(PoisonError::into_inner);
        ConnectionStatus {
            connection: inner.connection,
            disconnected_for: inner.disconnected_at.map(|at| at.elapsed()),
        }
    }

    /// Whether TCP keepalive is configured on the connection right now. `false` only once
    /// `configure_keepalive` has actually failed on some connection attempt (see
    /// [`Supervisor::run`]) -- see [`ConnectionState::new`] for why it starts `true`.
    ///
    /// This matters because when it is `false`, the bridge is silently back to the freeze bug
    /// keepalive exists to prevent: a refbox that dies without closing the connection (power
    /// loss, a cable pulled) will not be noticed at all, on this connection, until something else
    /// eventually breaks it. `configure_keepalive` failing is logged to stderr, which is invisible
    /// to an operator running a compiled program with no terminal in view -- the status page
    /// (Task 7) is what actually surfaces it.
    pub fn keepalive_active(&self) -> bool {
        self.keepalive_active.load(Ordering::SeqCst)
    }

    /// `pub(crate)`, not `pub`: outside this module, the only legitimate writer is
    /// [`Supervisor::run`]. Visible at the crate level (rather than private to this module) so
    /// `server`'s own tests can drive a table-serving test into the `Connected` state directly,
    /// without needing a real socket for scenarios that have nothing to do with connection
    /// lifecycle itself. Clears any previously-recorded drop instant in the same write -- see
    /// [`ConnectionStatus`]'s doc for why this must happen together with the state change, not as
    /// a separate step a caller (or a background poller) might observe only later.
    pub(crate) fn set_connected(&self) {
        let mut inner = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        inner.connection = Connection::Connected;
        inner.disconnected_at = None;
    }

    /// See [`ConnectionState::set_connected`]'s visibility note -- the same reasoning applies.
    /// Records the drop instant in the same write as the state change (see [`ConnectionStatus`]'s
    /// doc), and only on the actual transition into `Disconnected` -- a repeated call while
    /// already disconnected leaves the original instant alone, or the reported duration would
    /// reset every time this were called rather than growing from the real drop.
    pub(crate) fn set_disconnected(&self) {
        let mut inner = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        if inner.connection != Connection::Disconnected {
            inner.disconnected_at = Some(Instant::now());
        }
        inner.connection = Connection::Disconnected;
    }

    /// See [`ConnectionState::set_connected`]'s visibility note -- the same reasoning applies.
    /// Also used directly by `server`'s and `status`'s own tests to exercise the "unavailable"
    /// display without forcing a genuine OS-level keepalive failure, which cannot be done
    /// portably (see [`configure_keepalive`]'s doc).
    pub(crate) fn set_keepalive_active(&self) {
        self.keepalive_active.store(true, Ordering::SeqCst);
    }

    /// See [`ConnectionState::set_keepalive_active`]'s doc -- the same reasoning applies.
    pub(crate) fn set_keepalive_unavailable(&self) {
        self.keepalive_active.store(false, Ordering::SeqCst);
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Owns a refbox feed connection: connects, configures keepalive, forwards every snapshot to `tx`
/// in arrival order, and reconnects -- after `RECONNECT_DELAY` -- on any kind of loss: a refused
/// or failed connect, the refbox closing the stream, an I/O error, or a peer the keepalive probes
/// above have given up on. Publishes every one of those events to `connection` as they happen --
/// see [`Connection`] for why this, and not message timing, is the only thing allowed to decide
/// whether the bridge is "in contact". Also publishes whether `configure_keepalive` actually
/// succeeded on the current connection, via
/// [`ConnectionState::set_keepalive_active`]/[`ConnectionState::set_keepalive_unavailable`] --
/// re-evaluated on every (re)connect, since a failure on one attempt does not necessarily mean the
/// next one will fail too.
///
/// A malformed line (`FeedError::Parse`) is not a connection loss: it is logged and reading
/// continues on the same connection, exactly as `SnapshotReader` already reports it, and
/// `connection` is left untouched.
pub struct Supervisor;

impl Supervisor {
    /// Runs forever, reconnecting as needed. The only way it stops is `tx`'s corresponding
    /// receiver being dropped, which makes a send fail and ends the loop -- there is no other exit
    /// path, by design: silence on this feed is often legitimate (see the module doc), so nothing
    /// here ever gives up on a refbox that simply hasn't sent anything in a while.
    pub async fn run<A>(
        addr: A,
        tx: mpsc::UnboundedSender<GameSnapshot>,
        connection: ConnectionState,
    ) where
        A: ToSocketAddrs,
    {
        loop {
            let stream = match TcpStream::connect(&addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("could not connect to the refbox feed: {e}");
                    sleep(RECONNECT_DELAY).await;
                    continue;
                }
            };
            // This fires the instant the TCP handshake completes -- before the refbox's replayed
            // snapshot (its current game state, sent to every new client immediately on connect;
            // see `refbox/src/app/update_sender.rs:606-630`) has actually been read and parsed.
            // So there is a real, but narrow, window in which a request could see `connected:
            // "true"` while `state::LiveState` still holds whatever it held before this
            // reconnect (a stale value after a long outage, or the startup-seeded default on the
            // very first connect). Because the refbox sends that replay unprompted and
            // immediately -- no request from the bridge required -- this window is milliseconds,
            // not the seconds-to-tens-of-seconds scale the rest of this design reasons about, and
            // closing it would need no refbox change. Left open deliberately rather than adding
            // complexity (e.g. delaying `set_connected` until the first snapshot arrives, which
            // would just reintroduce a data-arrival dependency this design otherwise avoids) for a
            // gap this small.
            connection.set_connected();

            if let Err(e) = configure_keepalive(&stream) {
                eprintln!("could not configure TCP keepalive on the refbox feed connection: {e}");
                connection.set_keepalive_unavailable();
            } else {
                connection.set_keepalive_active();
            }

            let mut snapshots = SnapshotReader::new(LineLimited::new(stream));
            loop {
                match snapshots.next().await {
                    Some(Ok(snapshot)) => {
                        if tx.send(snapshot).is_err() {
                            // Nobody is listening any more; there is nothing left to run for.
                            return;
                        }
                    }
                    Some(Err(FeedError::Parse(e))) => {
                        eprintln!("could not parse a snapshot from the refbox feed: {e}");
                    }
                    Some(Err(FeedError::Io(e))) => {
                        eprintln!("lost the refbox feed connection: {e}");
                        connection.set_disconnected();
                        break;
                    }
                    None => {
                        eprintln!("the refbox closed the feed connection");
                        connection.set_disconnected();
                        break;
                    }
                }
            }

            sleep(RECONNECT_DELAY).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use futures::StreamExt;
    use serde_json::Value;
    use tokio::{io::AsyncWriteExt, net::TcpListener};
    use uwh_common::{
        bundles::{BlackWhiteBundle, OptColorBundle},
        color::Color,
        game_snapshot::{
            GamePeriod, Infraction, InfractionSnapshot, PenaltySnapshot, PenaltyTime,
            TimeoutSnapshot,
        },
    };

    use super::*;

    /// Real snapshots captured from a live refbox on 2026-08-26 (see the phase-1 plan's ruling on
    /// this fixture). One JSON object per line: between games, first half at several points
    /// (a goal, penalties building up, a foul, a warning, a timeout), half time, second half, and
    /// a between-games state with expired penalties.
    const FIXTURE: &str = include_str!("../tests/fixtures/feed-capture.jsonl");

    /// Fetch one line of the fixture by its 0-based position.
    fn fixture_line(n: usize) -> &'static str {
        FIXTURE
            .lines()
            .nth(n)
            .unwrap_or_else(|| panic!("fixture is missing line {n}"))
    }

    /// A test double that hands back exactly the byte chunks it is given, at most one (partial)
    /// chunk per `poll_read` call, so a test can control precisely how a message is split across
    /// reads. An exhausted queue reports end-of-stream (a zero-byte read), matching a closed
    /// connection.
    struct ChunkedReader {
        chunks: VecDeque<Vec<u8>>,
    }

    impl ChunkedReader {
        fn new(chunks: Vec<Vec<u8>>) -> Self {
            Self {
                chunks: chunks.into(),
            }
        }
    }

    impl AsyncRead for ChunkedReader {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if let Some(chunk) = self.chunks.front_mut() {
                let n = chunk.len().min(buf.remaining());
                buf.put_slice(&chunk[..n]);
                chunk.drain(..n);
                if chunk.is_empty() {
                    self.chunks.pop_front();
                }
            }
            Poll::Ready(Ok(()))
        }
    }

    /// The expected decode of `fixture_line(7)`: the 794-byte (with its newline) full-combination
    /// message — five penalties across both teams (including a `TotalDismissal`), a foul, a
    /// warning and an active timeout. This is the largest message in the fixture.
    fn line8_expected() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 10,
            timeout: Some(TimeoutSnapshot::Black(30)),
            scores: BlackWhiteBundle { black: 1, white: 0 },
            penalties: BlackWhiteBundle {
                black: vec![
                    PenaltySnapshot {
                        player_number: 1,
                        time: PenaltyTime::Seconds(25),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 2,
                        time: PenaltyTime::Seconds(85),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(265),
                        infraction: Infraction::Unknown,
                    },
                ],
                white: vec![
                    PenaltySnapshot {
                        player_number: 4,
                        time: PenaltyTime::Seconds(64),
                        infraction: Infraction::Unknown,
                    },
                    PenaltySnapshot {
                        player_number: 7,
                        time: PenaltyTime::TotalDismissal,
                        infraction: Infraction::Unknown,
                    },
                ],
            },
            warnings: BlackWhiteBundle {
                black: vec![InfractionSnapshot {
                    player_number: Some(6),
                    infraction: Infraction::IllegalAdvancement,
                }],
                white: vec![],
            },
            fouls: OptColorBundle {
                black: vec![InfractionSnapshot {
                    player_number: Some(3),
                    infraction: Infraction::StickInfringement,
                }],
                equal: vec![],
                white: vec![],
            },
            is_old_game: true,
            game_number: "1".to_string(),
            next_game_number: "2".to_string(),
            event_id: None,
            recent_goal: None,
            next_period_len_secs: Some(20),
            conf_pause_time: None,
        }
    }

    #[tokio::test]
    async fn one_message_per_newline_is_parsed() {
        let line = fixture_line(7);
        assert_eq!(
            line.len() + 1,
            794,
            "fixture_line(7) should be the 794-byte (with newline) full-combination message"
        );

        let reader = ChunkedReader::new(vec![format!("{line}\n").into_bytes()]);
        let mut stream = SnapshotReader::new(reader);

        let snapshot = stream
            .next()
            .await
            .expect("stream ended before yielding a snapshot")
            .expect("line should parse as a GameSnapshot");
        assert_eq!(snapshot, line8_expected());

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn several_messages_in_one_read_are_all_parsed() {
        let joined = format!(
            "{}\n{}\n{}\n",
            fixture_line(0),
            fixture_line(1),
            fixture_line(2)
        );
        // A single chunk, so the reader must find all three lines from one underlying read.
        let reader = ChunkedReader::new(vec![joined.into_bytes()]);
        let mut stream = SnapshotReader::new(reader);

        let first = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse");
        assert_eq!(first.current_period, GamePeriod::BetweenGames);
        assert_eq!(first.secs_in_period, 885);

        let second = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse");
        assert_eq!(second.secs_in_period, 90);

        let third = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse");
        assert_eq!(third.secs_in_period, 81);
        assert_eq!(third.recent_goal, Some((Color::Black, 6)));

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn a_message_split_across_two_reads_is_parsed_once_whole() {
        let bytes = format!("{}\n", fixture_line(3)).into_bytes();
        let midpoint = bytes.len() / 2;
        let (first_half, second_half) = bytes.split_at(midpoint);
        let reader = ChunkedReader::new(vec![first_half.to_vec(), second_half.to_vec()]);
        let mut stream = SnapshotReader::new(reader);

        let snapshot = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse");
        assert_eq!(snapshot.secs_in_period, 65);
        assert_eq!(
            snapshot.penalties.white[0],
            PenaltySnapshot {
                player_number: 4,
                time: PenaltyTime::Seconds(119),
                infraction: Infraction::Unknown,
            }
        );

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn a_message_far_larger_than_any_plausible_buffer_is_parsed() {
        // Start from a real captured message and keep appending real-shaped foul entries until
        // it exceeds 8 KB, well past the 1024-byte buffer that breaks the existing overlay.
        let mut value: Value =
            serde_json::from_str(fixture_line(7)).expect("fixture line 7 is valid JSON");

        let mut extra_fouls = 0usize;
        let mut line = serde_json::to_string(&value).expect("re-serializes");
        while line.len() <= 8192 {
            value["fouls"]["black"]
                .as_array_mut()
                .expect("fouls.black is an array")
                .push(serde_json::json!({
                    "player_number": (extra_fouls % 100) as u8,
                    "infraction": "Unknown",
                }));
            extra_fouls += 1;
            line = serde_json::to_string(&value).expect("re-serializes");
        }
        line.push('\n');
        assert!(
            line.len() > 8192,
            "test setup should have produced a message over 8 KB"
        );

        // Feed it back in small, deliberately arbitrary pieces so it can only be assembled from
        // many reads, never one.
        let chunks: Vec<Vec<u8>> = line.into_bytes().chunks(500).map(<[u8]>::to_vec).collect();
        assert!(
            chunks.len() > 1,
            "test setup should require more than one read"
        );
        let reader = ChunkedReader::new(chunks);
        let mut stream = SnapshotReader::new(reader);

        let snapshot = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("oversized message should still parse");
        assert_eq!(snapshot.fouls.black.len(), 1 + extra_fouls);

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn a_malformed_line_is_reported_and_does_not_desync_the_stream() {
        let joined = format!(
            "{}\nthis is not json\n{}\n",
            fixture_line(0),
            fixture_line(1)
        );
        let reader = ChunkedReader::new(vec![joined.into_bytes()]);
        let mut stream = SnapshotReader::new(reader);

        let first = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse");
        assert_eq!(first.secs_in_period, 885);

        let bad = stream.next().await.expect("stream ended early");
        assert!(
            matches!(bad, Err(FeedError::Parse(_))),
            "malformed line should be reported as a parse error, got {bad:?}"
        );

        let second = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse the line after the bad one");
        assert_eq!(second.secs_in_period, 90);

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn a_truncated_final_line_at_eof_is_not_reported_as_a_parse_failure() {
        let mut bytes = format!("{}\n", fixture_line(0)).into_bytes();
        // A fragment of a second message, cut off before its closing newline: as if the
        // connection dropped mid-write.
        bytes.extend_from_slice(&fixture_line(1).as_bytes()[..20]);
        let reader = ChunkedReader::new(vec![bytes]);
        let mut stream = SnapshotReader::new(reader);

        let first = stream
            .next()
            .await
            .expect("stream ended early")
            .expect("should parse");
        assert_eq!(first.secs_in_period, 885);

        // The trailing fragment is silently dropped, not surfaced as an error.
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn a_peer_with_no_newline_past_the_cap_is_reported_as_a_connection_error() {
        // One byte over the cap, with no newline anywhere in it.
        let payload = vec![b'x'; MAX_UNTERMINATED_BYTES + 1];
        let reader = ChunkedReader::new(vec![payload]);
        let mut stream = SnapshotReader::new(LineLimited::new(reader));

        let result = stream.next().await.expect("stream ended before erroring");
        assert!(
            matches!(result, Err(FeedError::Io(_))),
            "an unterminated peer past the cap should surface as a connection-level error, got \
             {result:?}"
        );
    }

    #[tokio::test]
    async fn keepalive_is_actually_applied_to_the_connected_socket() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");

        let accept = tokio::spawn(async move { listener.accept().await });
        let stream = TcpStream::connect(addr)
            .await
            .expect("connect to the local listener");
        accept
            .await
            .expect("accept task should not panic")
            .expect("accept should succeed");

        configure_keepalive(&stream).expect("keepalive should be configurable on a real socket");

        // Read the options back from the OS via getsockopt, rather than trusting that the setter
        // call above didn't error -- this is the only way to actually prove the settings landed on
        // the socket, since a genuinely half-open connection can't be produced reliably in a unit
        // test.
        let sock = SockRef::from(&stream);
        assert!(
            sock.keepalive().expect("read SO_KEEPALIVE"),
            "SO_KEEPALIVE should be enabled"
        );
        assert_eq!(
            sock.tcp_keepalive_time().expect("read TCP_KEEPIDLE"),
            KEEPALIVE_IDLE
        );
        assert_eq!(
            sock.tcp_keepalive_interval().expect("read TCP_KEEPINTVL"),
            KEEPALIVE_INTERVAL
        );
        assert_eq!(
            sock.tcp_keepalive_retries().expect("read TCP_KEEPCNT"),
            KEEPALIVE_RETRIES
        );
    }

    #[tokio::test]
    async fn snapshots_arriving_on_the_stream_reach_the_channel_in_order() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");

        let (tx, mut rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(Supervisor::run(addr, tx, ConnectionState::new()));

        let (mut refbox_side, _) = listener.accept().await.expect("accept");
        let payload = format!("{}\n{}\n", fixture_line(0), fixture_line(1));
        refbox_side
            .write_all(payload.as_bytes())
            .await
            .expect("write to the accepted connection");

        let first = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("first snapshot should arrive")
            .expect("channel should not be closed");
        assert_eq!(first.secs_in_period, 885);

        let second = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("second snapshot should arrive")
            .expect("channel should not be closed");
        assert_eq!(second.secs_in_period, 90);

        handle.abort();
    }

    #[tokio::test]
    async fn a_closed_connection_triggers_a_reconnect_instead_of_ending_the_task() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");

        let (tx, mut rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(Supervisor::run(addr, tx, ConnectionState::new()));

        let (first_connection, _) = listener.accept().await.expect("first accept");
        // Closed with no data at all -- a clean close (the refbox process exiting normally), not
        // a hang. The keepalive-detected silent disappearance is a different case, and can't be
        // reliably produced in a unit test (see the module doc); this test covers the "stream
        // just ends" path through the very same reconnect logic.
        drop(first_connection);

        // A second accept only succeeds if the supervisor noticed the loss and reconnected; if
        // reconnection were broken (e.g. the outer loop failing to run again), this would hang
        // until the timeout and fail the test.
        let (mut second_connection, _) =
            tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .expect("supervisor should have reconnected after the connection closed")
                .expect("second accept should succeed");

        // Prove the reconnected connection is actually being used, not just accepted and ignored.
        let payload = format!("{}\n", fixture_line(0));
        second_connection
            .write_all(payload.as_bytes())
            .await
            .expect("write to the reconnected connection");

        let snapshot = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("should receive a snapshot over the reconnected connection")
            .expect("channel should not be closed");
        assert_eq!(snapshot.secs_in_period, 885);

        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn a_refused_connection_retries_instead_of_exiting() {
        let probe = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let addr = probe.local_addr().expect("local_addr");
        drop(probe); // nothing is listening at `addr` any more; every connect is refused

        let (tx, _rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(Supervisor::run(addr, tx, ConnectionState::new()));

        // Let several retry cycles elapse on the paused clock. Tokio auto-advances a paused clock
        // to the next pending timer once every task is idle and only a timer is outstanding, so
        // this does not actually wait five real seconds -- it resolves as soon as both this sleep
        // and the supervisor's own retry timers have played out virtually.
        sleep(RECONNECT_DELAY * 5).await;

        assert!(
            !handle.is_finished(),
            "supervisor should keep retrying a refused connection rather than exit"
        );

        handle.abort();
    }

    // ---------------------------------------------------------------- Connection / ConnectionState
    //
    // These prove the connection state is driven by the connection itself, not by message
    // timing. `a_connection_that_has_never_succeeded_reports_never_connected_not_disconnected` in
    // particular is the direct test of the `NeverConnected`/`Disconnected` distinction the phase-1
    // plan's Task 10 notes call for: collapsing them into one "not connected" state would lose
    // the information a status display needs to tell "hasn't found a refbox yet" apart from
    // "found one, then lost it".

    /// Polls `connection` until it reports `target`, failing the test rather than hanging forever
    /// if it never does. Used instead of a fixed sleep-then-assert because how long the supervisor
    /// takes to notice and publish a transition is not exactly predictable (it depends on how the
    /// async runtime happens to schedule the read), and a fixed sleep long enough to never flake
    /// would make every test using it needlessly slow.
    async fn wait_for(connection: &ConnectionState, target: Connection) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while connection.get() != target {
            assert!(
                tokio::time::Instant::now() < deadline,
                "connection state did not reach {target:?} in time, currently {:?}",
                connection.get()
            );
            sleep(Duration::from_millis(5)).await;
        }
    }

    #[test]
    fn a_fresh_connection_state_reports_never_connected() {
        assert_eq!(ConnectionState::new().get(), Connection::NeverConnected);
    }

    // -------------------------------------------------------------------------- keepalive_active
    //
    // A genuine OS-level keepalive configuration failure can't be produced portably in a unit
    // test (see `configure_keepalive`'s doc), so these exercise the same `pub(crate)` setters
    // `Supervisor::run` itself calls, proving the flag's storage and the getter/setter wiring
    // independent of ever actually forcing the OS to refuse. The Task 7 report covers the
    // `status` module's own test of what the page renders in each state.

    #[test]
    fn a_fresh_connection_state_reports_keepalive_active() {
        assert!(
            ConnectionState::new().keepalive_active(),
            "keepalive should be assumed active before any connection attempt has been made -- \
             see ConnectionState::new's doc"
        );
    }

    #[test]
    fn marking_keepalive_unavailable_is_reflected_by_the_getter() {
        let connection = ConnectionState::new();
        connection.set_keepalive_unavailable();
        assert!(!connection.keepalive_active());
    }

    #[test]
    fn marking_keepalive_active_again_after_unavailable_is_reflected_by_the_getter() {
        let connection = ConnectionState::new();
        connection.set_keepalive_unavailable();
        assert!(!connection.keepalive_active());

        connection.set_keepalive_active();
        assert!(
            connection.keepalive_active(),
            "a later successful configure_keepalive call must be able to clear an earlier failure"
        );
    }

    #[tokio::test]
    async fn a_successful_connect_flips_a_previously_unavailable_keepalive_back_to_active() {
        // An end-to-end proof through the real `Supervisor::run` path (rather than poking the
        // setters directly, as the tests above do): on an ordinary CI runner, configuring
        // keepalive on a fresh loopback socket succeeds, so this proves the wiring between
        // `configure_keepalive`'s success path and `ConnectionState` actually runs.
        //
        // Starting from `set_keepalive_unavailable()` (not the default) is load-bearing, not
        // decoration -- see the Task 7 fix report for the full transcript. The flag defaults to
        // `true`, so a version of this test that never forced it `false` first would pass
        // identically whether or not `Supervisor::run`'s success arm
        // (`connection.set_keepalive_active()`) existed at all: deleting that line entirely would
        // leave the flag exactly where it started (`true`) and this test green regardless. Only
        // by starting from a known `false` does a passing assertion actually prove the success
        // arm ran.
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");

        let (tx, _rx) = mpsc::unbounded_channel();
        let connection = ConnectionState::new();
        connection.set_keepalive_unavailable();
        assert!(
            !connection.keepalive_active(),
            "test setup should start from a known false, not the default true"
        );

        let handle = tokio::spawn(Supervisor::run(addr, tx, connection.clone()));

        let _accepted = listener.accept().await.expect("accept");
        wait_for(&connection, Connection::Connected).await;

        assert!(
            connection.keepalive_active(),
            "a successful connect on a platform that supports TCP keepalive should flip an \
             earlier failure back to active, not merely leave a default alone"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn a_successful_connect_reports_connected_before_any_message_ever_arrives() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");

        let (tx, _rx) = mpsc::unbounded_channel();
        let connection = ConnectionState::new();
        let handle = tokio::spawn(Supervisor::run(addr, tx, connection.clone()));

        // Accept the connection but never write anything to it -- liveness must be reported from
        // the TCP connection itself, not from having received a first message.
        let _accepted = listener.accept().await.expect("accept");

        wait_for(&connection, Connection::Connected).await;

        handle.abort();
    }

    #[tokio::test]
    async fn a_lost_connection_reports_disconnected() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");

        let (tx, _rx) = mpsc::unbounded_channel();
        let connection = ConnectionState::new();
        let handle = tokio::spawn(Supervisor::run(addr, tx, connection.clone()));

        let (accepted, _) = listener.accept().await.expect("accept");
        // Wait for Connected first, so the transition this test actually exercises is genuinely
        // Connected -> Disconnected, not NeverConnected -> Disconnected.
        wait_for(&connection, Connection::Connected).await;

        drop(accepted); // a clean close, same as the reconnect test above -- EOF, not a hang

        wait_for(&connection, Connection::Disconnected).await;

        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn a_connection_that_has_never_succeeded_reports_never_connected_not_disconnected() {
        let probe = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let addr = probe.local_addr().expect("local_addr");
        drop(probe); // nothing is listening at `addr` any more; every connect is refused

        let (tx, _rx) = mpsc::unbounded_channel();
        let connection = ConnectionState::new();
        let handle = tokio::spawn(Supervisor::run(addr, tx, connection.clone()));

        // Several retry cycles, same reasoning as `a_refused_connection_retries_instead_of_exiting`
        // above: this resolves virtually, not after five real seconds.
        sleep(RECONNECT_DELAY * 5).await;

        assert_eq!(
            connection.get(),
            Connection::NeverConnected,
            "a connection that has never once succeeded must stay distinguishable from one that \
             connected and was then lost -- it must never report Disconnected"
        );

        handle.abort();
    }
}
