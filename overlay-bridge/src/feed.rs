//! Reads a refbox's network feed: one JSON-encoded `GameSnapshot` per line, terminated by `\n`.
//!
//! The refbox does not frame messages by length, only by newline, and a single message can be
//! larger than any one `read()` call returns. This reader buffers bytes until a full line is
//! available, so a message's size never determines whether it can be parsed. This matters because
//! the existing `overlay` crate reads into a fixed 1024-byte buffer without looking for the
//! newline: once a message exceeds that buffer, its reads desynchronise and every subsequent
//! message fails to parse. Nothing here bounds message size.

use std::{
    fmt, io,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::io::{AsyncRead, ReadBuf};
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

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use futures::StreamExt;
    use serde_json::Value;
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
}
