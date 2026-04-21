//! Background health-check task and retry loop.
//!
//! The decision logic (when to fire a health check, when to retry an item,
//! what cadence to use) is encapsulated in `HealthDecisionState` and is
//! unit-testable without any async runtime or I/O. The async side
//! (`spawn`, `run_task`, `PortalTaskIo`) drives that decision logic from
//! a Tokio task; see `BackgroundTaskHandle` for the UI-side integration
//! surface.

use std::time::{Duration, Instant};

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::sync::mpsc;

use super::HealthState;
use super::PortalEvent;
use super::is_item_stuck;
use super::queue::{QueueFile, QueuedItem};

pub const GREEN_CADENCE: Duration = Duration::from_secs(5 * 60);
pub const DEGRADED_CADENCE: Duration = Duration::from_secs(15);

/// How often `run_task` wakes to re-evaluate cadence and drain commands.
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub fn is_item_retry_eligible(item: &QueuedItem, now: OffsetDateTime) -> bool {
    if is_item_stuck(item, now) {
        return false; // Stuck items wait for operator action.
    }
    match item.last_attempt_at {
        None => true,
        Some(last) => (now - last) >= TimeDuration::seconds(15),
    }
}

/// What the background loop should do on its next wakeup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextAction {
    /// Wait until `at` and then re-evaluate.
    SleepUntil(Instant),
    /// Fire a verify_token health check.
    HealthCheck,
    /// Retry a queued item (by index into the queue's `items` vec).
    RetryItem(usize),
}

#[derive(Debug, Clone)]
pub struct HealthDecisionState {
    pub current_health: HealthState,
}

impl HealthDecisionState {
    pub fn next_cadence(&self) -> Duration {
        match self.current_health {
            HealthState::Green => GREEN_CADENCE,
            HealthState::Yellow | HealthState::Red => DEGRADED_CADENCE,
        }
    }

    /// Is a new health check due given the time since the last successful
    /// interaction? `None` means no successful interaction has ever
    /// happened — so a health check is always due.
    pub fn is_health_check_due(&self, elapsed_since_last: Option<Duration>) -> bool {
        match elapsed_since_last {
            None => true,
            Some(elapsed) => elapsed >= self.next_cadence(),
        }
    }
}

/// Commands from the UI side of PortalManager to the background task.
#[derive(Debug, Clone)]
pub enum PortalCommand {
    VerifyNow,
    ItemEnqueued(super::ItemId),
    RetryItem(super::ItemId),
    /// Refresh the task's view of the queue. Sent by `PortalManager`
    /// after every queue mutation so the task knows which items to
    /// attempt on its next tick.
    QueueUpdated(QueueFile),
    Shutdown,
}

pub struct BackgroundTaskHandle {
    pub command_tx: mpsc::Sender<PortalCommand>,
    pub event_rx: mpsc::Receiver<PortalEvent>,
}

pub fn spawn(io: impl PortalTaskIo + Send + Sync + 'static) -> BackgroundTaskHandle {
    let (command_tx, command_rx) = mpsc::channel(64);
    let (event_tx, event_rx) = mpsc::channel(64);
    tokio::spawn(run_task(io, command_rx, event_tx));
    BackgroundTaskHandle {
        command_tx,
        event_rx,
    }
}

#[async_trait::async_trait]
pub trait PortalTaskIo {
    async fn verify_token(&self) -> Result<(), PortalCallError>;
    async fn post_scores(&self, item: &QueuedItem) -> Result<(), PortalCallError>;
    async fn post_stats(&self, item: &QueuedItem) -> Result<(), PortalCallError>;
}

#[derive(Debug)]
pub enum PortalCallError {
    /// The portal call did not succeed. We cannot distinguish a conflict
    /// (409), token-expiry (401), or network/5xx failure from the current
    /// uwh-common API — all non-success outcomes collapse to this one
    /// variant. See ADR 011 amendment (2026-04-21) for the rationale.
    Failed(String),
}

async fn run_task(
    io: impl PortalTaskIo,
    mut command_rx: mpsc::Receiver<PortalCommand>,
    event_tx: mpsc::Sender<PortalEvent>,
) {
    use tokio::time::Instant as TokioInstant;
    use tokio::time::sleep;

    let mut last_success: Option<TokioInstant> = None;
    let mut current_health = HealthState::Green;
    let mut queue_snapshot = QueueFile::empty();

    loop {
        tokio::select! {
            _ = sleep(POLL_INTERVAL) => {
                // Tick: try each eligible queued item, then health-check if due.
                let now = OffsetDateTime::now_utc();
                for item in &queue_snapshot.items {
                    if is_item_retry_eligible(item, now)
                        && attempt_item(&io, item, &event_tx).await
                    {
                        last_success = Some(TokioInstant::now());
                    }
                }

                let elapsed = last_success
                    .map(|t| TokioInstant::now().saturating_duration_since(t));
                let state = HealthDecisionState { current_health };
                if state.is_health_check_due(elapsed) {
                    let _ = event_tx
                        .send(PortalEvent::HealthChanged(HealthState::Yellow))
                        .await;
                    match io.verify_token().await {
                        Ok(()) => {
                            last_success = Some(TokioInstant::now());
                            current_health = HealthState::Green;
                            let _ = event_tx
                                .send(PortalEvent::HealthChanged(HealthState::Green))
                                .await;
                        }
                        Err(_) => {
                            current_health = HealthState::Red;
                            let _ = event_tx
                                .send(PortalEvent::HealthChanged(HealthState::Red))
                                .await;
                        }
                    }
                }
            }
            cmd = command_rx.recv() => {
                match cmd {
                    Some(PortalCommand::Shutdown) | None => break,
                    Some(PortalCommand::VerifyNow) => { last_success = None; }
                    Some(PortalCommand::ItemEnqueued(_)) | Some(PortalCommand::RetryItem(_)) => {
                        last_success = None;
                    }
                    Some(PortalCommand::QueueUpdated(new_queue)) => {
                        queue_snapshot = new_queue;
                    }
                }
            }
        }
    }
}

/// Attempt to submit a single queued item (scores + stats). The portal
/// API collapses all non-success outcomes (409 conflict, 401 token
/// expired, 5xx, network) into a single error, so we treat any `Err`
/// identically: leave the item on the queue for the next retry tick,
/// and emit `ItemUpdated` so the UI can refresh the row. Only a full
/// success (both scores and stats posted) emits `ItemResolved`.
///
/// Returns `true` iff both portal calls succeeded. The caller uses this
/// to decide whether to stamp `last_success`; advancing `last_success`
/// on a failed attempt would suppress the cadence-driven
/// `verify_token` health check and leave the indicator stuck at Green
/// during a silent portal outage.
async fn attempt_item(
    io: &impl PortalTaskIo,
    item: &QueuedItem,
    event_tx: &mpsc::Sender<PortalEvent>,
) -> bool {
    let score_result = io.post_scores(item).await;
    if score_result.is_err() {
        let _ = event_tx
            .send(PortalEvent::ItemUpdated(item.id.clone()))
            .await;
        return false;
    }
    match io.post_stats(item).await {
        Ok(()) => {
            let _ = event_tx
                .send(PortalEvent::ItemResolved(item.id.clone()))
                .await;
            true
        }
        Err(_) => {
            let _ = event_tx
                .send(PortalEvent::ItemUpdated(item.id.clone()))
                .await;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ItemId;
    use super::*;

    fn mk_queue_item(attempts: u32) -> QueuedItem {
        QueuedItem {
            id: ItemId {
                event_id: "e".into(),
                game_number: "G1".into(),
            },
            black_score: 0,
            white_score: 0,
            stats: "{}".into(),
            queued_at: OffsetDateTime::now_utc(),
            attempts,
            last_attempt_at: None,
            force: false,
        }
    }

    #[test]
    fn green_cadence_is_five_minutes() {
        let s = HealthDecisionState {
            current_health: HealthState::Green,
        };
        assert_eq!(s.next_cadence(), GREEN_CADENCE);
    }

    #[test]
    fn yellow_and_red_cadence_is_fifteen_seconds() {
        for h in [HealthState::Yellow, HealthState::Red] {
            let s = HealthDecisionState { current_health: h };
            assert_eq!(s.next_cadence(), DEGRADED_CADENCE);
        }
    }

    #[test]
    fn first_ever_check_is_due_immediately() {
        let s = HealthDecisionState {
            current_health: HealthState::Green,
        };
        assert!(s.is_health_check_due(None));
    }

    #[test]
    fn green_check_not_due_if_cadence_not_elapsed() {
        let s = HealthDecisionState {
            current_health: HealthState::Green,
        };
        assert!(!s.is_health_check_due(Some(Duration::from_secs(60))));
    }

    #[test]
    fn green_check_due_after_cadence() {
        let s = HealthDecisionState {
            current_health: HealthState::Green,
        };
        assert!(s.is_health_check_due(Some(GREEN_CADENCE)));
    }

    #[test]
    fn degraded_check_due_after_fifteen_seconds() {
        let s = HealthDecisionState {
            current_health: HealthState::Red,
        };
        assert!(s.is_health_check_due(Some(Duration::from_secs(15))));
    }

    #[test]
    fn young_item_with_zero_attempts_is_eligible_for_retry() {
        let item = mk_queue_item(0);
        assert!(is_item_retry_eligible(&item, OffsetDateTime::now_utc()));
    }

    #[test]
    fn stuck_item_is_not_eligible_for_auto_retry() {
        let mut item = mk_queue_item(4);
        // Queued 31 minutes ago — past the 30-minute stuck threshold.
        item.queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(31);
        assert!(!is_item_retry_eligible(&item, OffsetDateTime::now_utc()));
    }

    #[test]
    fn item_with_recent_attempt_respects_cadence() {
        let mut item = mk_queue_item(1);
        let now = OffsetDateTime::now_utc();
        item.last_attempt_at = Some(now);
        // 5 seconds later — still within the 15s cadence.
        assert!(!is_item_retry_eligible(
            &item,
            now + TimeDuration::seconds(5)
        ));
        // 15 seconds later — eligible again.
        assert!(is_item_retry_eligible(
            &item,
            now + TimeDuration::seconds(15)
        ));
    }

    use std::sync::{Arc, Mutex};

    struct FakeIo {
        verify_results: Mutex<Vec<Result<(), PortalCallError>>>,
        verify_count: Arc<std::sync::atomic::AtomicU32>,
        scores_results: Mutex<Vec<Result<(), PortalCallError>>>,
        scores_count: Arc<std::sync::atomic::AtomicU32>,
        stats_results: Mutex<Vec<Result<(), PortalCallError>>>,
        stats_count: Arc<std::sync::atomic::AtomicU32>,
    }

    #[async_trait::async_trait]
    impl PortalTaskIo for FakeIo {
        async fn verify_token(&self) -> Result<(), PortalCallError> {
            self.verify_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.verify_results.lock().unwrap();
            if v.is_empty() { Ok(()) } else { v.remove(0) }
        }
        async fn post_scores(&self, _: &QueuedItem) -> Result<(), PortalCallError> {
            self.scores_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.scores_results.lock().unwrap();
            if v.is_empty() { Ok(()) } else { v.remove(0) }
        }
        async fn post_stats(&self, _: &QueuedItem) -> Result<(), PortalCallError> {
            self.stats_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.stats_results.lock().unwrap();
            if v.is_empty() { Ok(()) } else { v.remove(0) }
        }
    }

    /// Drain all currently-queued events without blocking.
    fn drain_events(rx: &mut mpsc::Receiver<PortalEvent>) -> Vec<PortalEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            out.push(ev);
        }
        out
    }

    // Smoke test: confirms `spawn` starts a task that wakes on the poll
    // interval, calls `verify_token`, and shuts down cleanly. Because the
    // task starts with `last_success = None`, the very first tick is
    // cadence-due regardless of whether `VerifyNow` was sent — so this
    // test does NOT distinguish the `VerifyNow`-forced-check path from
    // the first-tick path. That distinction requires injected initial
    // state and lands in Task 10's test suite.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn background_task_performs_health_check_on_first_tick() {
        let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: count.clone(),
            scores_results: Mutex::new(vec![]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        };
        let handle = spawn(io);

        handle
            .command_tx
            .send(PortalCommand::VerifyNow)
            .await
            .unwrap();

        // Two rounds of (advance + yield) are needed: the first wakes the
        // select's timer arm and lets `verify_token`'s future be polled;
        // the second gives that future a chance to resolve before we read
        // the counter.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;

        assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 1);

        handle
            .command_tx
            .send(PortalCommand::Shutdown)
            .await
            .unwrap();
    }

    /// Build a `QueueFile` with a single freshly-queued item (no prior
    /// attempts), which makes that item immediately retry-eligible by
    /// `is_item_retry_eligible`.
    fn queue_with_one_eligible_item() -> QueueFile {
        QueueFile {
            version: QueueFile::CURRENT_VERSION,
            items: vec![mk_queue_item(0)],
        }
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn eligible_item_triggers_scores_then_stats_and_emits_resolved_on_success() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        // Provide a successful `verify_token` in case the cadence-due
        // health check also fires in this tick — we don't want the task
        // to block on a missing result.
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Ok(())]),
            stats_count: stats_count.clone(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated(queue))
            .await
            .unwrap();

        // Let the QueueUpdated command be received before the first tick.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(scores_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(stats_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemResolved(id) if id == &expected_id)),
            "expected ItemResolved event for {expected_id:?}, got {events:?}"
        );

        handle
            .command_tx
            .send(PortalCommand::Shutdown)
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn scores_failure_emits_item_updated_and_skips_stats() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Err(PortalCallError::Failed("boom".into()))]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![]),
            stats_count: stats_count.clone(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated(queue))
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(scores_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(stats_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated(id) if id == &expected_id)),
            "expected ItemUpdated event for {expected_id:?}, got {events:?}"
        );

        handle
            .command_tx
            .send(PortalCommand::Shutdown)
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn queue_updated_command_replaces_snapshot_before_next_tick() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Ok(())]),
            stats_count: stats_count.clone(),
        };
        let handle = spawn(io);

        // First: send an empty queue and advance past a tick. No
        // retry-eligible item exists, so no scores/stats calls.
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated(QueueFile::empty()))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;

        assert_eq!(scores_count.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(stats_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        // Second: replace the snapshot with a queue containing one
        // eligible item, advance past the next tick, and confirm the
        // task saw the updated snapshot.
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated(queue_with_one_eligible_item()))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(scores_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(stats_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        handle
            .command_tx
            .send(PortalCommand::Shutdown)
            .await
            .unwrap();
    }
}
