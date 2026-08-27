//! Reads the refbox's game-state feed: one JSON-encoded `GameSnapshot` per line, terminated by
//! `\n` (`refbox/src/app/update_sender.rs`).
//!
//! The refbox frames messages only by that newline, and a single message can be larger than any one
//! read returns — a line grows with every foul, warning and penalty recorded, from roughly 400
//! bytes with nothing recorded to past 1024 at about a dozen entries. This reader therefore
//! accumulates bytes until a full line is available and keeps whatever is left over, so how the
//! reads happen to be chunked, and how large a message is, never decide whether it can be parsed.
//!
//! Splitting on newlines is safe because `serde_json`'s compact output never emits a raw newline
//! inside a value — a newline in a string is escaped as the two characters `\n` — so every `\n` on
//! the wire is a message boundary. (`to_string_pretty` would break that, but the refbox does not
//! use it.)
//!
//! This replaces a reader that took up to 1024 bytes in one read and parsed whatever it got. Once a
//! message exceeded that buffer, its tail stayed in the socket, every later read started mid-line,
//! and nothing resynchronised: the overlay stopped updating for the rest of the game rather than
//! dropping a single update.

use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};

/// How many bytes to ask the connection for per read. This is only I/O granularity, not a limit on
/// message size: a longer message is simply assembled from more than one read.
const READ_CHUNK_BYTES: usize = 4096;

/// How many bytes may arrive with no line ending at all before the read fails.
///
/// This is **not** a message-size limit — a terminated line is parsed however long it is. It guards
/// against the far end not being a refbox feed at all: pointed at the wrong port, a service that
/// holds the connection open and never sends a newline would otherwise grow `line_buf` until the
/// machine ran out of memory. The overlay reconnects on any read error, so tripping this behaves
/// exactly like a lost connection. It sits about 1000x above the largest line a real refbox can
/// produce, and two orders of magnitude above the largest line these tests exercise.
const MAX_UNTERMINATED_BYTES: usize = 1024 * 1024;

/// Splits a byte stream into the refbox feed's newline-terminated lines.
///
/// Create one per connection: a connection that dies part-way through a line leaves bytes buffered
/// here, and those must never be prepended to the next connection's first line.
pub struct FeedReader<R> {
    reader: R,
    /// Bytes received from the feed that have not yet been handed out as complete lines.
    line_buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> FeedReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: Vec::new(),
        }
    }

    /// Returns the next complete line, without its trailing newline.
    ///
    /// `Ok(None)` means the stream ended; any bytes left part-way through a line end with it, since
    /// there is no way to tell a truncated message from nothing at all. `Err(_)` is a read failure
    /// or [`MAX_UNTERMINATED_BYTES`] being exceeded — the caller treats both as a lost connection.
    pub async fn next_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut chunk = [0u8; READ_CHUNK_BYTES];
        loop {
            if let Some(line) = self.take_line() {
                return Ok(Some(line));
            }

            // No complete line is buffered, so everything in `line_buf` is one unterminated line.
            if self.line_buf.len() > MAX_UNTERMINATED_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "feed sent over {MAX_UNTERMINATED_BYTES} bytes with no line ending; \
                         the far end does not look like a refbox"
                    ),
                ));
            }

            let read = self.reader.read(&mut chunk).await?;
            if read == 0 {
                return Ok(None);
            }
            self.line_buf.extend_from_slice(&chunk[..read]);
        }
    }

    /// Removes and returns the first complete line in `line_buf`, without its trailing newline.
    /// Leaves `line_buf` untouched when no full line has arrived yet.
    fn take_line(&mut self) -> Option<Vec<u8>> {
        let newline = self.line_buf.iter().position(|&b| b == b'\n')?;
        let mut line: Vec<u8> = self.line_buf.drain(..=newline).collect();
        line.pop(); // drop the trailing '\n'
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        io,
        pin::Pin,
        task::{Context, Poll},
    };

    use tokio::{
        io::{AsyncRead, AsyncWriteExt, ReadBuf},
        net::{TcpListener, TcpStream},
    };
    use uwh_common::{
        bundles::BlackWhiteBundle,
        game_snapshot::{
            GamePeriod, GameSnapshot, Infraction, InfractionSnapshot, PenaltySnapshot, PenaltyTime,
        },
    };

    use super::*;

    /// A stand-in for a TCP connection that hands back exactly the byte chunks it was given, at
    /// most one per read, so a test can control precisely how a line is split across reads. An
    /// exhausted queue reports end-of-stream, matching a closed connection.
    struct ChunkedReader {
        chunks: VecDeque<Vec<u8>>,
    }

    impl ChunkedReader {
        /// Every chunk must be non-empty. Filling zero bytes is how this double reports end of
        /// stream, so an empty chunk would silently mean "the connection closed" rather than
        /// "nothing arrived this time".
        fn new(chunks: Vec<Vec<u8>>) -> Self {
            assert!(
                chunks.iter().all(|chunk| !chunk.is_empty()),
                "an empty chunk is indistinguishable from end of stream"
            );
            Self {
                chunks: chunks.into(),
            }
        }
    }

    impl AsyncRead for ChunkedReader {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            let Some(chunk) = this.chunks.front_mut() else {
                // Nothing left to hand out: a zero-byte read, i.e. end of stream.
                return Poll::Ready(Ok(()));
            };
            let take = chunk.len().min(buf.remaining());
            buf.put_slice(&chunk[..take]);
            chunk.drain(..take);
            if chunk.is_empty() {
                this.chunks.pop_front();
            }
            Poll::Ready(Ok(()))
        }
    }

    /// A snapshot carrying `count` fouls, warnings and penalties, so its serialised form grows past
    /// the 1024-byte buffer the old reader used. This mirrors what actually makes a real feed line
    /// long: fouls and warnings accumulate for the whole game and are never culled.
    fn snapshot_with(count: usize) -> GameSnapshot {
        let mut snapshot = GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            secs_in_period: 421,
            scores: BlackWhiteBundle { black: 3, white: 2 },
            ..Default::default()
        };
        for i in 0..count {
            let player = (i % 20) as u8 + 1;
            let entry = InfractionSnapshot {
                player_number: Some(player),
                infraction: Infraction::UnsportsmanlikeConduct,
            };
            snapshot.fouls.black.push(entry.clone());
            snapshot.warnings.white.push(entry);
            snapshot.penalties.black.push(PenaltySnapshot {
                player_number: player,
                time: PenaltyTime::Seconds(60),
                infraction: Infraction::IllegalAdvancement,
            });
        }
        snapshot
    }

    /// One feed line exactly as the refbox sends it: the snapshot's JSON plus a trailing newline.
    fn line_for(snapshot: &GameSnapshot) -> Vec<u8> {
        let mut line = serde_json::to_vec(snapshot).unwrap();
        line.push(b'\n');
        line
    }

    fn parse(line: &[u8]) -> GameSnapshot {
        serde_json::from_slice(line).expect("line should parse as a snapshot")
    }

    #[tokio::test]
    async fn an_oversized_line_parses_and_the_next_line_still_parses() {
        let big = snapshot_with(40);
        let big_line = line_for(&big);
        assert!(
            big_line.len() > 1024,
            "fixture must exceed the old 1024-byte buffer, was {} bytes",
            big_line.len()
        );
        assert!(
            big_line.len() > READ_CHUNK_BYTES,
            "fixture must need more than one read, was {} bytes",
            big_line.len()
        );

        let small = snapshot_with(0);
        let mut reader = FeedReader::new(ChunkedReader::new(vec![big_line, line_for(&small)]));

        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), big);
        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), small);
        assert!(reader.next_line().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn two_lines_in_one_read_both_parse_and_the_next_line_still_parses() {
        let first = snapshot_with(1);
        let second = snapshot_with(2);
        let third = snapshot_with(3);

        let mut coalesced = line_for(&first);
        coalesced.extend_from_slice(&line_for(&second));

        let mut reader = FeedReader::new(ChunkedReader::new(vec![coalesced, line_for(&third)]));

        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), first);
        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), second);
        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), third);
    }

    #[tokio::test]
    async fn a_line_split_across_reads_parses_once_complete_and_the_next_line_still_parses() {
        let split = snapshot_with(2);
        let line = line_for(&split);
        let cut = line.len() / 2;
        let follow_up = snapshot_with(5);

        let mut reader = FeedReader::new(ChunkedReader::new(vec![
            line[..cut].to_vec(),
            line[cut..].to_vec(),
            line_for(&follow_up),
        ]));

        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), split);
        assert_eq!(
            parse(&reader.next_line().await.unwrap().unwrap()),
            follow_up
        );
    }

    #[tokio::test]
    async fn a_corrupted_line_leaves_the_reader_lined_up_for_the_next_line() {
        let good = snapshot_with(3);
        let mut chunk = b"{not valid json at all}\n".to_vec();
        chunk.extend_from_slice(&line_for(&good));

        let mut reader = FeedReader::new(ChunkedReader::new(vec![chunk]));

        let corrupt = reader.next_line().await.unwrap().unwrap();
        assert!(serde_json::from_slice::<GameSnapshot>(&corrupt).is_err());
        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), good);
    }

    #[tokio::test]
    async fn a_stream_ending_part_way_through_a_line_reports_the_end() {
        let line = line_for(&snapshot_with(2));
        let partial = line[..line.len() / 2].to_vec();

        let mut reader = FeedReader::new(ChunkedReader::new(vec![partial]));

        assert!(reader.next_line().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn run_on_input_with_no_line_ending_fails_instead_of_growing_without_bound() {
        let run_on = vec![b'x'; MAX_UNTERMINATED_BYTES + READ_CHUNK_BYTES];

        let mut reader = FeedReader::new(ChunkedReader::new(vec![run_on]));

        let error = reader
            .next_line()
            .await
            .expect_err("run-on input should be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    /// The other tests drive a stand-in reader so they can control how bytes are chunked. This one
    /// runs over a real TCP connection, the type the overlay actually uses, so nothing about the
    /// fix depends on the stand-in behaving like a socket.
    #[tokio::test]
    async fn an_oversized_line_survives_a_real_tcp_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let big = snapshot_with(40);
        let small = snapshot_with(1);
        let big_line = line_for(&big);
        let small_line = line_for(&small);

        let refbox = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket.write_all(&big_line).await.unwrap();
            socket.write_all(&small_line).await.unwrap();
            socket.shutdown().await.unwrap();
        });

        let mut reader = FeedReader::new(TcpStream::connect(address).await.unwrap());

        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), big);
        assert_eq!(parse(&reader.next_line().await.unwrap().unwrap()), small);
        assert!(reader.next_line().await.unwrap().is_none());

        refbox.await.unwrap();
    }
}
