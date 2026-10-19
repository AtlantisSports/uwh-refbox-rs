//! The background task that sends the in-play stats list to the portal.
//!
//! One loop, strictly sequential, like the live sender: each send is awaited before the next
//! decision, so at most one is in flight, and the frames arrive on a `watch` channel that holds
//! only the newest one. Unlike the live sender, nothing is sent on a timer: a list is sent once
//! when it changes, and a failed send is retried every [`StatsTiming::retry`] until the portal
//! accepts it, a newer list replaces it, or the frame goes away (the game ended, or the send
//! conditions stopped holding). Nothing is saved to disk.
//!
//! This task never touches the score/stats upload queue or its health indicator.

use super::{
    StatsFrame,
    sender::{FailureLog, LogWording, classify, keyed_send, write_log},
};
use crate::portal_manager::{SharedAccessKeys, SharedUwhPortalClient};
use std::time::Duration;
use tokio::{sync::watch, time::Instant};

/// How long to wait before retrying a failed send, and how long one send may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatsTiming {
    pub retry: Duration,
    pub request_timeout: Duration,
}

pub const STATS_TIMING: StatsTiming = StatsTiming {
    retry: Duration::from_secs(15),
    request_timeout: Duration::from_secs(3),
};

/// Start the stats sender. It runs until `frames`' sender is dropped.
pub fn spawn_stats_sender(
    frames: watch::Receiver<Option<StatsFrame>>,
    client: SharedUwhPortalClient,
    keys: SharedAccessKeys,
    timing: StatsTiming,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run(frames, client, keys, timing))
}

async fn run(
    mut frames: watch::Receiver<Option<StatsFrame>>,
    client: SharedUwhPortalClient,
    keys: SharedAccessKeys,
    timing: StatsTiming,
) {
    // The frame the portal last accepted. A frame that is the same list is not sent again.
    let mut accepted: Option<StatsFrame> = None;
    let mut log = FailureLog::new(LogWording::STATS);

    loop {
        // Cloned out at once: the borrow holds the channel's lock and must not cross an await.
        let current = frames.borrow_and_update().clone();
        let Some(frame) = current else {
            // The game ended or the conditions stopped holding: any pending retry is dropped,
            // and a list that comes back is sent again.
            accepted = None;
            if frames.changed().await.is_err() {
                return;
            }
            continue;
        };

        if accepted.as_ref().is_some_and(|a| a.same_list(&frame)) {
            if frames.changed().await.is_err() {
                return;
            }
            continue;
        }

        let outcome = match keyed_send(&client, &keys, &frame.site, &frame.event, |c, key| {
            c.put_live_game_stats(
                key,
                &frame.event,
                &frame.game,
                &frame.list.body,
                timing.request_timeout,
            )
        }) {
            Ok(send) => send.await.map_err(|why| classify(why.as_ref())),
            Err(skip) => Err(skip),
        };
        let delivered = outcome.is_ok();
        write_log(log.record(outcome, Instant::now()));
        if delivered {
            accepted = Some(frame);
            continue;
        }

        // Failed, or not sendable (no key, the site changed): try again after the retry
        // interval, or at once with whatever replaced this frame, whichever comes first. Either
        // way the newest frame is read again, never this one.
        let changed = tokio::select! {
            changed = frames.changed() => changed,
            () = tokio::time::sleep(timing.retry) => Ok(()),
        };
        if changed.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::sender::{Failure, LogLine};
    use super::super::test_portal::{Portal, Reply, answer_at_once, event, portal, shared};
    use super::*;
    use crate::tournament_manager::RunningStats;
    use std::sync::atomic::Ordering;

    /// A frame for game 5 whose list has `key`. The body differs from the key, as the real one
    /// does, so a test can tell which was sent.
    fn frame(site: &str, key: &str) -> StatsFrame {
        StatsFrame {
            site: site.to_string(),
            event: event(),
            game: "5".to_string(),
            list: RunningStats {
                body: format!("[{{\"list\":\"{key}\"}}]"),
                key: key.to_string(),
            },
        }
    }

    fn timing(retry_ms: u64, timeout_ms: u64) -> StatsTiming {
        StatsTiming {
            retry: Duration::from_millis(retry_ms),
            request_timeout: Duration::from_millis(timeout_ms),
        }
    }

    fn start(
        portal: &Portal,
        first: Option<StatsFrame>,
        timing: StatsTiming,
    ) -> (
        watch::Sender<Option<StatsFrame>>,
        tokio::task::JoinHandle<()>,
    ) {
        let (client, keys) = shared(&portal.site, &portal.site);
        let (tx, rx) = watch::channel(first);
        (tx, spawn_stats_sender(rx, client, keys, timing))
    }

    /// How long to wait for a request that is expected: generous, as in the live sender's tests,
    /// so a stalled runner does not fail a test that is merely slow.
    const WAIT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn sends_put_to_stats_path_with_body() {
        let mut portal = portal(answer_at_once).await;
        let sent = frame(&portal.site, "A");
        let (_tx, _task) = start(&portal, Some(sent.clone()), timing(1000, 1500));
        let got = portal.next(WAIT).await.expect("a request");
        assert_eq!(got.method, "PUT");
        assert_eq!(got.path, "/api/events/1889-B/live-games/5/stats");
        assert_eq!(got.authorization.as_deref(), Some("Bearer KEY-LIVE"));
        assert_eq!(got.body, sent.list.body);
    }

    #[tokio::test]
    async fn unchanged_list_not_resent() {
        let mut portal = portal(answer_at_once).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(100, 1500));
        portal.next(WAIT).await.expect("a first request");
        // The same list rebuilt: same key, a body whose `occurredOn` jittered.
        let mut rebuilt = frame(&portal.site, "A");
        rebuilt.list.body.push(' ');
        tx.send(Some(rebuilt)).unwrap();
        tx.send(Some(frame(&portal.site, "A"))).unwrap();
        let rest = portal.all_within(Duration::from_millis(1000)).await;
        assert_eq!(rest.len(), 0, "an unchanged list is not sent again");
    }

    #[tokio::test]
    async fn changed_list_sent_once() {
        let mut portal = portal(answer_at_once).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(100, 1500));
        portal.next(WAIT).await.expect("a first request");
        let changed = frame(&portal.site, "B");
        tx.send(Some(changed.clone())).unwrap();
        let rest = portal.all_within(Duration::from_millis(1500)).await;
        assert_eq!(rest.len(), 1, "a changed list is sent once");
        assert_eq!(rest[0].body, changed.list.body);
    }

    #[tokio::test]
    async fn failure_retried_after_retry_interval() {
        let mut portal = portal(|i, _| {
            if i == 0 {
                Reply::Status("500 Internal Server Error")
            } else {
                Reply::After(Duration::ZERO)
            }
        })
        .await;
        let sent = frame(&portal.site, "A");
        let (_tx, _task) = start(&portal, Some(sent.clone()), timing(100, 1500));
        let first = portal.next(WAIT).await.expect("a first request");
        let rest = portal.all_within(Duration::from_millis(1500)).await;
        assert_eq!(
            rest.len(),
            1,
            "500 then 204: exactly one retry, then nothing"
        );
        assert_eq!(rest[0].body, sent.list.body);
        // The retry waits out the interval after the failure; a stall can only lengthen this.
        let gap = rest[0].at - first.at;
        assert!(
            gap >= Duration::from_millis(90),
            "retried {gap:?} after the first; the retry interval is 100 ms"
        );
    }

    /// Review Focus 5: a list change while sends are failing goes out on the next attempt, and
    /// the stale list is never sent again.
    #[tokio::test]
    async fn retry_carries_newest_list() {
        let mut portal = portal(|i, _| {
            if i < 3 {
                Reply::Status("500 Internal Server Error")
            } else {
                Reply::After(Duration::ZERO)
            }
        })
        .await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(150, 1500));
        portal.next(WAIT).await.expect("a first request");
        let newest = frame(&portal.site, "B");
        tx.send(Some(newest.clone())).unwrap();
        let rest = portal.all_within(Duration::from_millis(2000)).await;
        assert_eq!(rest.len(), 3, "two more failures, then accepted");
        for r in &rest {
            assert_eq!(r.body, newest.list.body, "a retry carried a stale list");
        }
    }

    #[tokio::test]
    async fn none_drops_pending_retry() {
        let mut portal = portal(|_, _| Reply::Status("500 Internal Server Error")).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(300, 1500));
        portal.next(WAIT).await.expect("a first request");
        tx.send(None).unwrap();
        let rest = portal.all_within(Duration::from_millis(1200)).await;
        assert_eq!(rest.len(), 0, "no retry once the frame is gone");
    }

    #[tokio::test]
    async fn accepted_list_stops_retries() {
        let mut portal = portal(answer_at_once).await;
        let (_tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(100, 1500));
        portal.next(WAIT).await.expect("a first request");
        let rest = portal.all_within(Duration::from_millis(1000)).await;
        assert_eq!(rest.len(), 0, "an accepted list is not retried");
    }

    #[tokio::test]
    async fn stalled_send_abandoned_by_timeout() {
        let mut portal = portal(|i, _| {
            if i == 0 {
                Reply::Hang
            } else {
                Reply::After(Duration::ZERO)
            }
        })
        .await;
        let sent = frame(&portal.site, "A");
        let (_tx, _task) = start(&portal, Some(sent.clone()), timing(100, 1000));
        let first = portal.next(WAIT).await.expect("a first request");
        let second = portal.next(WAIT).await.expect("a retry");
        // Expected about 1.1 s (the timeout, then the retry). Ignoring the timeout leaves it to
        // the client's own 10 s one.
        let gap = second.at - first.at;
        assert!(
            gap >= Duration::from_millis(600) && gap < Duration::from_millis(3500),
            "retry {gap:?} after the hung send; the timeout is 1 s"
        );
        assert_eq!(second.body, sent.list.body);
    }

    #[tokio::test]
    async fn never_two_in_flight() {
        let mut portal = portal(|_, _| Reply::After(Duration::from_millis(1500))).await;
        let (tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(100, 4000));
        portal.next(WAIT).await.expect("a first request");
        for key in ["B", "C", "D", "E"] {
            tokio::time::sleep(Duration::from_millis(80)).await;
            tx.send(Some(frame(&portal.site, key))).unwrap();
        }
        let rest = portal.all_within(Duration::from_millis(3000)).await;
        assert!(!rest.is_empty(), "sending went on");
        assert_eq!(portal.max_in_flight.load(Ordering::SeqCst), 1);
        // Coalesced: the changes made during the first send go out as one, the newest.
        assert_eq!(rest[0].body, frame(&portal.site, "E").list.body);
    }

    /// Review Focus 4: a portal without the endpoint answers 404 all game. The send keeps being
    /// retried, and the failure is logged once per quiet window, not once per retry.
    #[tokio::test]
    async fn not_found_is_retried_and_logged_once_per_window() {
        let mut portal = portal(|_, _| Reply::Status("404 Not Found")).await;
        let (_tx, _task) = start(&portal, Some(frame(&portal.site, "A")), timing(100, 1500));
        portal.next(WAIT).await.expect("a first request");
        let rest = portal.all_within(Duration::from_millis(1500)).await;
        assert!(rest.len() >= 2, "a 404 is retried: {} retries", rest.len());

        // One attempt every 15 s for the 10-minute window: one warning. The next window: one more.
        let start = Instant::now();
        let mut log = FailureLog::new(LogWording::STATS);
        let not_found = || Failure::Rejected {
            status: "404 Not Found".to_string(),
            body: "boom".to_string(),
        };
        let lines: Vec<_> = (0..40)
            .filter_map(|i| log.record(Err(not_found()), start + Duration::from_secs(15 * i)))
            .collect();
        assert_eq!(lines.len(), 1, "{lines:?}");
        let LogLine::Warn(text) = &lines[0] else {
            panic!("expected a warning, got {lines:?}");
        };
        assert!(text.contains("will be retried"), "{text}");
        assert!(text.contains("404 Not Found"), "{text}");
        assert!(matches!(
            log.record(Err(not_found()), start + Duration::from_secs(600)),
            Some(LogLine::Warn(_))
        ));
    }

    #[tokio::test]
    async fn conditions_return_resends_current_list() {
        let mut portal = portal(answer_at_once).await;
        let sent = frame(&portal.site, "A");
        let (tx, _task) = start(&portal, Some(sent.clone()), timing(100, 1500));
        portal.next(WAIT).await.expect("a first request");
        tx.send(None).unwrap();
        assert_eq!(
            portal.all_within(Duration::from_millis(500)).await.len(),
            0,
            "nothing is sent while there is no frame"
        );
        tx.send(Some(sent.clone())).unwrap();
        let again = portal.next(WAIT).await.expect("the list sent again");
        assert_eq!(again.body, sent.list.body);
    }

    #[tokio::test]
    async fn ends_when_the_channel_closes() {
        let portal = portal(answer_at_once).await;
        let (tx, task) = start(&portal, None, timing(100, 150));
        drop(tx);
        tokio::time::timeout(WAIT, task)
            .await
            .expect("the task ends")
            .unwrap();
    }
}
