//! Finds refboxes on the local network, and tells a real one from any other open port.
//!
//! # Why this exists at all
//!
//! The refbox never displays its own network address anywhere in the application (spec §10.3), so
//! "just type the address" means sending a broadcast volunteer out of the refbox and into the
//! machine's operating-system settings to go looking for it. Spec §5.3 makes checking the network
//! the normal path and typing an address the fallback -- not the other way round.
//!
//! # What makes a candidate a refbox
//!
//! **A refbox replays its current game state the instant anything connects**
//! (`refbox/src/app/update_sender.rs:606-630`). That single fact is what turns a port scan into
//! refbox discovery: an open port proves only that *something* is listening, but a port that
//! immediately says something the bridge can parse as a `GameSnapshot` is a refbox, and the very
//! same message says which game it is showing. So a probe here connects, reads exactly one
//! snapshot, labels the candidate from it, and closes -- see [`probe`].
//!
//! Nothing about this asks the refbox for anything, or needs the refbox to change (spec §4.2): a
//! probe is indistinguishable, from the refbox's side, from the bridge itself connecting.
//!
//! # Scanning is best-effort, by design
//!
//! Venue networks and Windows firewalls can both block a scan outright (spec §9.3), and a first
//! scan on Windows may raise a firewall prompt. A scan that finds nothing is therefore an ordinary
//! outcome, not an error: [`scan`] returns whatever answered and says nothing about the rest, and
//! typing an address by hand must always remain possible -- which is why [`probe`] is also what
//! the status page's manual-entry field uses to check an address before switching to it. One
//! mechanism, two front ends.
//!
//! # No new dependency
//!
//! Everything here is `tokio` (`net`, `time`), `futures`, and the existing feed reader --
//! [`SnapshotReader`] and [`LineLimited`] are reused rather than re-implemented, so a probe reads
//! a newline-framed snapshot by exactly the same rules, and with exactly the same guard against a
//! peer that streams bytes with no newline in them, as the live feed does.

use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::Duration,
};

use futures::StreamExt;
use tokio::{net::TcpStream, time::timeout};
use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};

use crate::feed::{LineLimited, RefboxAddress, SnapshotReader};

/// How long a single candidate gets to accept a connection *and* replay a snapshot before it is
/// written off as "not a refbox".
///
/// Deliberately generous relative to what a real refbox needs (it replays immediately, over a
/// local network, so a real one answers in milliseconds) and deliberately short relative to an
/// operator's patience: every candidate in a scan is probed at the same time, so this is very
/// nearly the whole scan's worst case, not a per-address cost. Two seconds keeps a full
/// 254-address sweep of a subnet where *nothing at all* answers inside the "few seconds" the plan
/// calls for.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// How many addresses a scan covers: the usable hosts of a /24, `.1` through `.254`.
pub const HOSTS_PER_SUBNET: usize = 254;

/// A confirmed refbox: where it is, and what it is showing right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub address: RefboxAddress,
    /// A human label built from the snapshot the refbox replayed on connect -- see [`label_for`].
    /// This is what the operator actually chooses by: two Pis at a two-court venue are told apart
    /// by the game on each, never by the last digits of an IP address.
    pub label: String,
}

/// Why a candidate address is not (or cannot be shown to be) a refbox. Each variant's
/// [`fmt::Display`] is a plain sentence for the status page -- the operator sees this text when an
/// address they typed does not work, and it has to tell them what to do next.
#[derive(Debug)]
pub enum ProbeError {
    /// Nothing accepted a connection there -- refused, unroutable, or the name did not resolve.
    Unreachable(std::io::Error),
    /// Something accepted the connection but sent nothing at all within [`PROBE_TIMEOUT`]. A
    /// refbox never does this: it replays the current game the instant anything connects.
    Silent,
    /// Something answered, but not with a game snapshot -- some other service on that port.
    NotARefbox,
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeError::Unreachable(e) => write!(
                f,
                "nothing answered there ({e}). Check the address, and that the refbox is switched \
                 on and on the same network as this computer"
            ),
            ProbeError::Silent => write!(
                f,
                "something is listening there, but it did not send a game — so it is not a \
                 refbox. Check the address"
            ),
            ProbeError::NotARefbox => write!(
                f,
                "something is listening there, but what it sent was not a game — so it is some \
                 other program, not a refbox. Check the address"
            ),
        }
    }
}

impl std::error::Error for ProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProbeError::Unreachable(e) => Some(e),
            ProbeError::Silent | ProbeError::NotARefbox => None,
        }
    }
}

/// Connects to `address`, reads the one snapshot a refbox replays on connect, and closes.
///
/// The connection is dropped as soon as that snapshot has been read (or `within` has elapsed), so
/// a probe costs a real refbox one connection lasting milliseconds -- which is exactly what the
/// bridge itself does on every reconnect, and what the existing overlay does continuously.
///
/// `within` covers the whole thing -- name resolution, the TCP handshake and the snapshot -- so a
/// probe cannot outlive it however it fails; [`PROBE_TIMEOUT`] is the value used everywhere in the
/// bridge itself, and it is a parameter only so this crate's own tests need not spend two real
/// seconds proving that a silent port is rejected.
pub async fn probe(address: &RefboxAddress, within: Duration) -> Result<Found, ProbeError> {
    match timeout(within, read_one_snapshot(address)).await {
        Ok(Ok(snapshot)) => Ok(Found {
            address: address.clone(),
            label: label_for(&snapshot),
        }),
        Ok(Err(ReadFailure::Connect(e))) => Err(ProbeError::Unreachable(e)),
        Ok(Err(ReadFailure::NotASnapshot)) => Err(ProbeError::NotARefbox),
        // Timed out: something accepted the connection (a refused one would have returned
        // `Connect` well inside the window) but never spoke.
        Err(_elapsed) => Err(ProbeError::Silent),
    }
}

/// The failure modes of [`read_one_snapshot`] that are distinguishable without a clock -- running
/// out of time is decided by [`probe`]'s own `timeout`, not in here.
enum ReadFailure {
    Connect(std::io::Error),
    /// End-of-stream, a read error, or a line that would not parse: all of them mean the same
    /// thing to a caller, which is that whatever is there is not a refbox.
    NotASnapshot,
}

async fn read_one_snapshot(address: &RefboxAddress) -> Result<GameSnapshot, ReadFailure> {
    let stream = TcpStream::connect((address.host.as_str(), address.port))
        .await
        .map_err(ReadFailure::Connect)?;

    // The same reader, and the same unterminated-input guard, as the live feed -- see the module
    // doc's "No new dependency" section.
    let mut snapshots = SnapshotReader::new(LineLimited::new(stream));
    match snapshots.next().await {
        Some(Ok(snapshot)) => Ok(snapshot),
        Some(Err(_)) | None => Err(ReadFailure::NotASnapshot),
    }
}

/// The label the operator picks by, built from the snapshot a candidate replayed:
///
/// ```text
/// Game 14 · Second Half · 3:47 · 2–1
/// Game 15 · Between Games
/// ```
///
/// Between games there is no meaningful clock or score to show -- the score belongs to a game that
/// has finished and the clock is counting down to the next one -- so those two parts are left off
/// rather than shown as something an operator might read as the current game's. A refbox that has
/// no game number yet (a fresh start with nothing loaded) simply loses the leading part, rather
/// than displaying "Game" with nothing after it.
pub fn label_for(snapshot: &GameSnapshot) -> String {
    let mut parts: Vec<String> = Vec::new();

    let game_number = snapshot.game_number();
    if !game_number.is_empty() {
        parts.push(format!("Game {game_number}"));
    }
    parts.push(snapshot.current_period.to_string());

    if snapshot.current_period != GamePeriod::BetweenGames {
        let secs = snapshot.secs_in_period;
        parts.push(format!("{}:{:02}", secs / 60, secs % 60));
        parts.push(format!(
            "{}–{}",
            snapshot.scores.black, snapshot.scores.white
        ));
    }

    parts.join(" · ")
}

/// Every address a scan of `subnet`'s /24 covers: `.1` through `.254`, in order. The fourth octet
/// of `subnet` is ignored, so any address *on* the network identifies it -- an operator can type
/// this computer's own address (which is what the status page pre-fills) rather than having to
/// know what "the network address" means.
///
/// `.0` (the network itself) and `.255` (its broadcast address) are not hosts and are never
/// probed.
fn scan_targets(subnet: Ipv4Addr) -> impl Iterator<Item = Ipv4Addr> {
    let [a, b, c, _] = subnet.octets();
    (1..=254u8).map(move |d| Ipv4Addr::new(a, b, c, d))
}

/// Checks every address on `subnet`'s /24 for a refbox listening on `port`, and returns the ones
/// that answered with a game, in address order.
///
/// Best-effort by design (see the module doc): an address that refuses the connection, is not
/// there at all, or turns out to be some other program is simply not in the result -- it never
/// fails the scan, because on a real venue network the overwhelming majority of the 254 addresses
/// are exactly that.
pub async fn scan(subnet: Ipv4Addr, port: u16) -> Vec<Found> {
    let targets: Vec<RefboxAddress> = scan_targets(subnet)
        .map(|ip| RefboxAddress::new(ip.to_string(), port))
        .collect();

    let mut found = probe_all(targets, |address| async move {
        probe(&address, PROBE_TIMEOUT).await.ok()
    })
    .await;

    // Address order, not the order they happened to answer in, so the list an operator is reading
    // does not reshuffle itself between one scan and the next.
    found.sort_by_key(|f| f.address.host.parse::<Ipv4Addr>().ok());
    found
}

/// Probes every target **at the same time**, and collects the ones that were refboxes.
///
/// The concurrency is the whole point, and it is why this is a named function rather than a line
/// inside [`scan`]: probed one after another, 254 addresses on a network that silently drops
/// packets would take 254 × [`PROBE_TIMEOUT`] -- over eight minutes -- where all at once takes
/// barely longer than a single probe. Split out from `scan` so that property can be tested for
/// what it is, with a stand-in probe and no network at all (`scan` itself can only be timed
/// against loopback, where every non-listening address refuses instantly and a sequential
/// implementation would look just as fast).
///
/// 254 sockets opened at once is well inside every platform's per-process descriptor limit, and
/// they are short-lived: each closes as soon as its probe resolves.
async fn probe_all<P, F>(targets: Vec<RefboxAddress>, probe_one: P) -> Vec<Found>
where
    P: Fn(RefboxAddress) -> F,
    F: Future<Output = Option<Found>>,
{
    futures::future::join_all(targets.into_iter().map(probe_one))
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// This computer's own IPv4 address on whichever network it would use to reach the outside world
/// -- the network the refbox is almost certainly on too, and so the sensible thing to pre-fill the
/// status page's scan field with.
///
/// Works by asking the operating system which local address it *would* use for a given
/// destination: a UDP socket is created and "connected", which chooses a route and assigns a local
/// address without sending a single packet. The destination is a TEST-NET-1 address (RFC 5737),
/// which is guaranteed never to be a real host, so nothing is contacted even in principle, and
/// nothing here depends on the venue having internet -- only on the computer having a route.
///
/// `None` when that cannot be answered (no route at all, or an IPv6-only network). That is not an
/// error: the scan field is then simply empty for the operator to fill in, and typing an address
/// by hand still works -- see the module doc.
pub fn local_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))).ok()?;
    // A literal address, not a name: nothing here may ever wait on a name lookup.
    socket
        .connect(SocketAddr::from((Ipv4Addr::new(192, 0, 2, 1), 9)))
        .ok()?;
    match socket.local_addr().ok()? {
        SocketAddr::V4(local) if is_usable_scan_source(*local.ip()) => Some(*local.ip()),
        _ => None,
    }
}

/// Whether an address is one a scan could sensibly be based on. Loopback would scan 127.x -- this
/// computer only -- and the unspecified address (`0.0.0.0`) names no network at all; either would
/// send the operator on a scan that could not possibly find their refbox, which is worse than an
/// empty field they can type into.
fn is_usable_scan_source(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified() && !ip.is_broadcast() && !ip.is_multicast()
}

/// The network to pre-fill the status page's scan field with: this computer's own address if that
/// can be worked out, otherwise the refbox address currently in use if it is an IPv4 address (a
/// stored address from a previous session is good evidence of which network the refboxes are on),
/// otherwise nothing.
pub fn suggested_scan_network(current: &RefboxAddress) -> String {
    suggestion_from(local_ipv4(), current)
}

/// The choice [`suggested_scan_network`] makes, with the one part that depends on the machine
/// passed in rather than looked up.
///
/// Split out because it could not otherwise be tested where it matters (Task 8 review, Important
/// 2). `local_ipv4()` returns an address on any machine with a default route and `None` on one
/// without -- so a test that called `suggested_scan_network` directly could only ever exercise
/// whichever branch that particular machine happened to take, and the `None` branch is precisely
/// the one that runs at a venue with no route to the outside world. Passing the local address in
/// makes all three outcomes reachable from a test on any machine.
///
/// `local` is expected to be already filtered by [`is_usable_scan_source`] (which is what
/// `local_ipv4` does); this function's own job is the fallback chain, not re-checking that.
fn suggestion_from(local: Option<Ipv4Addr>, current: &RefboxAddress) -> String {
    if let Some(local) = local {
        return local.to_string();
    }
    match current.host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) if is_usable_scan_source(ip) => ip.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::Ipv4Addr,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Instant,
    };

    use tokio::{io::AsyncWriteExt, net::TcpListener};
    use uwh_common::{bundles::BlackWhiteBundle, game_snapshot::GamePeriod};

    use super::*;

    /// A snapshot shaped like the plan's own worked example: `Game 14 · Second Half · 3:47 · 2–1`.
    fn second_half_snapshot() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::SecondHalf,
            secs_in_period: 227, // 3:47
            scores: BlackWhiteBundle { black: 2, white: 1 },
            game_number: "14".to_string(),
            ..Default::default()
        }
    }

    /// The plan's other worked example: `Game 15 · Between Games`. `is_old_game: false` with
    /// `BetweenGames` is what makes `GameSnapshot::game_number()` report the *upcoming* game
    /// (`next_game_number`) as the current one -- the refbox's own rule, not one invented here.
    fn between_games_snapshot() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 885,
            scores: BlackWhiteBundle { black: 7, white: 3 },
            game_number: "14".to_string(),
            next_game_number: "15".to_string(),
            is_old_game: false,
            ..Default::default()
        }
    }

    /// Binds a loopback listener that behaves like a refbox: it replays `snapshot` to every client
    /// the instant it connects, then leaves the connection open (exactly what a real refbox does
    /// -- it does not hang up, and it may then say nothing for a long time).
    ///
    /// Returns the address it is listening on. The accept loop runs until the test drops the
    /// returned handle.
    async fn fake_refbox(snapshot: GameSnapshot) -> (RefboxAddress, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let line = format!(
            "{}\n",
            serde_json::to_string(&snapshot).expect("GameSnapshot should serialize")
        );

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let line = line.clone();
                tokio::spawn(async move {
                    let _ = socket.write_all(line.as_bytes()).await;
                    // Hold the connection open, like a refbox with a stopped clock, until the
                    // other side closes it.
                    let mut sink = Vec::new();
                    let _ = tokio::io::AsyncReadExt::read_to_end(&mut socket, &mut sink).await;
                });
            }
        });

        (addr.into(), handle)
    }

    // ------------------------------------------------------------------------------- label_for

    #[test]
    fn a_live_game_is_labelled_with_its_number_period_clock_and_score() {
        assert_eq!(
            label_for(&second_half_snapshot()),
            "Game 14 · Second Half · 3:47 · 2–1"
        );
    }

    #[test]
    fn a_between_games_refbox_is_labelled_with_the_upcoming_game_and_no_clock_or_score() {
        // The clock and score are deliberately absent: between games the score belongs to the
        // game that just finished, and showing "7–3" beside "Game 15" would name the wrong game's
        // score. This snapshot carries a nonzero score and a running clock precisely so their
        // absence from the label is a real assertion rather than a vacuous one.
        assert_eq!(
            label_for(&between_games_snapshot()),
            "Game 15 · Between Games"
        );
    }

    #[test]
    fn a_refbox_with_no_game_number_is_still_labelled_readably() {
        let snapshot = GameSnapshot {
            game_number: String::new(),
            ..second_half_snapshot()
        };
        assert_eq!(label_for(&snapshot), "Second Half · 3:47 · 2–1");
    }

    // ----------------------------------------------------------------------------------- probe

    #[tokio::test]
    async fn a_refbox_that_replays_a_snapshot_is_found_and_labelled() {
        let (address, refbox) = fake_refbox(second_half_snapshot()).await;

        let found = probe(&address, PROBE_TIMEOUT)
            .await
            .expect("a server that replays a snapshot should be recognised as a refbox");

        assert_eq!(found.address, address);
        assert_eq!(found.label, "Game 14 · Second Half · 3:47 · 2–1");

        refbox.abort();
    }

    #[tokio::test]
    async fn a_port_that_accepts_but_sends_nothing_is_not_a_refbox() {
        // The case that makes this discovery rather than a port scan: something IS listening here,
        // and it accepts the connection, so anything judging by "did the connection succeed" would
        // report it as a refbox. It never speaks, so it is not one.
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let silent = tokio::spawn(async move {
            let held = listener.accept().await;
            // Hold the accepted connection open and say nothing at all.
            std::future::pending::<()>().await;
            drop(held);
        });

        let result = probe(&addr.into(), Duration::from_millis(300)).await;

        assert!(
            matches!(result, Err(ProbeError::Silent)),
            "an open port that never sends a snapshot must not be reported as a refbox, got \
             {result:?}"
        );

        silent.abort();
    }

    #[tokio::test]
    async fn a_port_that_answers_with_something_else_is_not_a_refbox() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener");
        let addr = listener.local_addr().expect("local_addr");
        let other_service = tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            // A web server, say, answering an unrecognised request.
            let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
        });

        let result = probe(&addr.into(), PROBE_TIMEOUT).await;

        assert!(
            matches!(result, Err(ProbeError::NotARefbox)),
            "a port that answers with something other than a game snapshot must not be reported \
             as a refbox, got {result:?}"
        );

        other_service.abort();
    }

    #[tokio::test]
    async fn a_refused_port_is_reported_as_unreachable_rather_than_hanging() {
        let probe_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let addr = probe_listener.local_addr().expect("local_addr");
        drop(probe_listener); // nothing is listening there any more

        // NOT `PROBE_TIMEOUT`, deliberately: on Windows, connecting to a refused loopback
        // port takes the OS itself about 2.0-2.05s to report `ConnectionRefused` (Windows
        // retries the SYN internally rather than sending an immediate RST the way
        // Linux/macOS do) -- measured directly on a Windows machine, not a guess. That is
        // *just* longer than `PROBE_TIMEOUT` (`Duration::from_secs(2)`), so this test's own
        // `timeout()` always won the race first on Windows, hiding the real
        // `ConnectionRefused` behind a `Silent` instead. Every other test in this file picks
        // its own short, test-appropriate window rather than reusing the production
        // constant (see `a_port_that_accepts_but_sends_nothing_is_not_a_refbox` two tests
        // up) -- this one is fixed to do the same, with margin past the measured OS latency.
        let result = probe(&addr.into(), Duration::from_secs(3)).await;

        assert!(
            matches!(result, Err(ProbeError::Unreachable(_))),
            "a refused connection should be reported as unreachable, got {result:?}"
        );
    }

    // ---------------------------------------------------------------------------- scan_targets

    #[test]
    fn a_scan_covers_every_host_of_the_subnet_and_neither_network_nor_broadcast() {
        let targets: Vec<Ipv4Addr> = scan_targets(Ipv4Addr::new(192, 168, 1, 37)).collect();

        assert_eq!(targets.len(), HOSTS_PER_SUBNET);
        assert_eq!(targets.first(), Some(&Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(targets.last(), Some(&Ipv4Addr::new(192, 168, 1, 254)));
        assert!(!targets.contains(&Ipv4Addr::new(192, 168, 1, 0)));
        assert!(!targets.contains(&Ipv4Addr::new(192, 168, 1, 255)));
    }

    #[test]
    fn any_address_on_the_network_identifies_the_same_scan() {
        // The operator types this computer's own address, not "the network address" -- a term a
        // broadcast volunteer has no reason to know.
        let from_host: Vec<Ipv4Addr> = scan_targets(Ipv4Addr::new(10, 0, 4, 200)).collect();
        let from_network: Vec<Ipv4Addr> = scan_targets(Ipv4Addr::new(10, 0, 4, 0)).collect();
        assert_eq!(from_host, from_network);
    }

    // ------------------------------------------------------------------------------------ scan

    #[tokio::test]
    async fn a_full_subnet_scan_finds_a_real_refbox_and_finishes_in_a_few_seconds() {
        // A real refbox on 127.0.0.1, and 253 loopback addresses with nothing on them. Finding
        // the planted refbox is what proves the scan actually probed rather than returning an
        // empty list quickly -- a scan that did nothing at all would also be fast.
        let (address, refbox) = fake_refbox(second_half_snapshot()).await;

        let started = Instant::now();
        let found = scan(Ipv4Addr::new(127, 0, 0, 1), address.port).await;
        let elapsed = started.elapsed();

        assert_eq!(
            found.len(),
            1,
            "exactly the planted refbox should have been found, got {found:?}"
        );
        assert_eq!(found[0].address, address);
        assert_eq!(found[0].label, "Game 14 · Second Half · 3:47 · 2–1");
        assert!(
            elapsed < Duration::from_secs(5),
            "a full 254-address scan should finish in a few seconds, took {elapsed:?}"
        );

        refbox.abort();
    }

    #[tokio::test]
    async fn addresses_that_refuse_the_connection_do_not_fail_the_scan() {
        // Every address in this scan refuses (nothing is listening on the reserved-then-released
        // port anywhere on loopback). The scan must come back empty and calm, not error.
        let probe_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local listener to reserve a port");
        let port = probe_listener.local_addr().expect("local_addr").port();
        drop(probe_listener);

        let found = scan(Ipv4Addr::new(127, 0, 0, 1), port).await;

        assert!(
            found.is_empty(),
            "nothing was listening anywhere, so nothing should be reported, got {found:?}"
        );
    }

    // ------------------------------------------------------------------------------- probe_all

    #[tokio::test]
    async fn every_address_is_probed_at_the_same_time_not_one_after_another() {
        // The real concurrency proof, with a stand-in probe instead of a network: each "probe"
        // takes 100ms, and there are 254 of them. Run one after another that is 25 seconds; run
        // together it is barely over 100ms. The assertion sits far below the sequential figure and
        // far above the concurrent one, so it cannot pass by accident on a slow machine, and it
        // cannot pass at all if the fan-out is ever replaced by a loop.
        let probed = Arc::new(AtomicUsize::new(0));
        let targets: Vec<RefboxAddress> = scan_targets(Ipv4Addr::new(192, 168, 1, 0))
            .map(|ip| RefboxAddress::new(ip.to_string(), 8000))
            .collect();
        assert_eq!(targets.len(), HOSTS_PER_SUBNET);

        let started = Instant::now();
        let found = probe_all(targets, |address| {
            let probed = Arc::clone(&probed);
            async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                probed.fetch_add(1, Ordering::SeqCst);
                // Report every other address as a refbox, so the collected result proves this
                // keeps the hits and discards the misses rather than simply returning everything.
                let last_octet: u8 = address
                    .host
                    .rsplit('.')
                    .next()
                    .and_then(|octet| octet.parse().ok())
                    .expect("test targets are dotted-quad addresses");
                (last_octet % 2 == 0).then(|| Found {
                    address,
                    label: "Game 1 · First Half · 1:00 · 0–0".to_string(),
                })
            }
        })
        .await;
        let elapsed = started.elapsed();

        assert_eq!(
            probed.load(Ordering::SeqCst),
            HOSTS_PER_SUBNET,
            "every address must be probed"
        );
        assert_eq!(found.len(), HOSTS_PER_SUBNET / 2);
        assert!(
            elapsed < Duration::from_secs(5),
            "254 probes of 100ms each must run together (~0.1s), not one after another (~25s); \
             took {elapsed:?}"
        );
    }

    // ------------------------------------------------ what to suggest scanning (local_ipv4)
    //
    // Every branch below runs on every machine, which is the point: whether this computer has a
    // route to the outside world decides which branch `suggested_scan_network` itself takes, and
    // the no-route branch -- a venue network with no way out, which is an ordinary thing at a pool
    // -- would otherwise never be exercised anywhere it will ever run.

    #[test]
    fn this_computers_own_address_is_what_gets_suggested_when_there_is_one() {
        assert_eq!(
            suggestion_from(
                Some(Ipv4Addr::new(192, 168, 4, 7)),
                &RefboxAddress::new("10.0.0.9", 8000)
            ),
            "192.168.4.7",
            "the computer's own network beats a remembered refbox address: it is where this \
             machine can actually see refboxes"
        );
    }

    #[test]
    fn with_no_local_address_the_refbox_already_in_use_names_the_network() {
        // The venue-with-no-route case. A refbox address remembered from a previous session is
        // good evidence of which network the refboxes are on.
        assert_eq!(
            suggestion_from(None, &RefboxAddress::new("10.0.0.9", 8000)),
            "10.0.0.9"
        );
    }

    #[test]
    fn with_nothing_usable_to_suggest_the_field_is_left_empty() {
        // A hostname is not a network, and loopback would scan this computer alone. Either would
        // send the operator on a search that could not possibly find their refbox, which is worse
        // than an empty field they can type into.
        for host in ["refbox.local", "127.0.0.1", "::1", ""] {
            assert_eq!(
                suggestion_from(None, &RefboxAddress::new(host, 8000)),
                "",
                "{host:?} is not a network to suggest scanning"
            );
        }
    }

    #[test]
    fn a_scan_source_has_to_be_an_address_a_refbox_could_be_near() {
        assert!(is_usable_scan_source(Ipv4Addr::new(192, 168, 1, 5)));
        assert!(is_usable_scan_source(Ipv4Addr::new(10, 0, 0, 9)));
        assert!(!is_usable_scan_source(Ipv4Addr::LOCALHOST));
        assert!(!is_usable_scan_source(Ipv4Addr::UNSPECIFIED));
        assert!(!is_usable_scan_source(Ipv4Addr::BROADCAST));
        assert!(!is_usable_scan_source(Ipv4Addr::new(224, 0, 0, 1)));
    }

    #[test]
    fn asking_this_computer_for_its_own_address_never_answers_with_a_useless_one() {
        // Whether there is an answer at all depends on the machine, so this asserts only what
        // holds either way: `local_ipv4` never hands back something `suggestion_from` would then
        // pass on as a network to scan. The three branches of that decision are covered above,
        // deterministically; this covers the filter actually being applied to the real lookup.
        if let Some(local) = local_ipv4() {
            assert!(
                is_usable_scan_source(local),
                "local_ipv4 must filter what it returns, got {local}"
            );
        }
    }
}
