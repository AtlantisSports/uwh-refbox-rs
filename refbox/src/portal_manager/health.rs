//! Background health-check task and retry loop.
//!
//! The decision logic (when to fire a health check, when to retry an item,
//! what cadence to use) is encapsulated in `HealthDecisionState` and is
//! unit-testable without any async runtime or I/O.

use std::time::{Duration, Instant};

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::sync::mpsc;

use super::HealthState;
use super::PortalEvent;
use super::is_item_stuck;
use super::queue::QueuedItem;

pub const GREEN_CADENCE: Duration = Duration::from_secs(5 * 60);
pub const DEGRADED_CADENCE: Duration = Duration::from_secs(15);

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
    pub last_successful_interaction: Option<Instant>,
    pub current_health: HealthState,
}

impl HealthDecisionState {
    pub fn next_cadence(&self) -> Duration {
        match self.current_health {
            HealthState::Green => GREEN_CADENCE,
            HealthState::Yellow | HealthState::Red => DEGRADED_CADENCE,
        }
    }

    /// Has the cadence elapsed since the last successful interaction?
    pub fn is_health_check_due(&self, now: Instant) -> bool {
        match self.last_successful_interaction {
            None => true, // First ever check — fire immediately.
            Some(last) => now.saturating_duration_since(last) >= self.next_cadence(),
        }
    }
}

/// Commands from the UI side of PortalManager to the background task.
#[derive(Debug, Clone)]
pub enum PortalCommand {
    VerifyNow,
    ItemEnqueued(super::ItemId),
    RetryItem(super::ItemId),
    Shutdown,
}

pub struct BackgroundTaskHandle {
    pub command_tx: mpsc::Sender<PortalCommand>,
    pub event_rx: mpsc::Receiver<PortalEvent>,
}

pub fn spawn(io: impl PortalTaskIo + Send + 'static) -> BackgroundTaskHandle {
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
    use tokio::time::{Duration as TokioDuration, sleep};

    let mut last_success: Option<Instant> = None;
    let mut current_health = HealthState::Green;

    loop {
        let sleep_for = TokioDuration::from_millis(2_000);

        tokio::select! {
            _ = sleep(sleep_for) => {
                let state = HealthDecisionState {
                    last_successful_interaction: last_success,
                    current_health,
                };
                if state.is_health_check_due(Instant::now()) {
                    let _ = event_tx.send(PortalEvent::HealthChanged(HealthState::Yellow)).await;
                    match io.verify_token().await {
                        Ok(()) => {
                            last_success = Some(Instant::now());
                            current_health = HealthState::Green;
                            let _ = event_tx.send(PortalEvent::HealthChanged(HealthState::Green)).await;
                        }
                        Err(_) => {
                            current_health = HealthState::Red;
                            let _ = event_tx.send(PortalEvent::HealthChanged(HealthState::Red)).await;
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
                }
            }
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
            last_successful_interaction: Some(Instant::now()),
            current_health: HealthState::Green,
        };
        assert_eq!(s.next_cadence(), GREEN_CADENCE);
    }

    #[test]
    fn yellow_and_red_cadence_is_fifteen_seconds() {
        for h in [HealthState::Yellow, HealthState::Red] {
            let s = HealthDecisionState {
                last_successful_interaction: Some(Instant::now()),
                current_health: h,
            };
            assert_eq!(s.next_cadence(), DEGRADED_CADENCE);
        }
    }

    #[test]
    fn first_ever_check_is_due_immediately() {
        let s = HealthDecisionState {
            last_successful_interaction: None,
            current_health: HealthState::Green,
        };
        assert!(s.is_health_check_due(Instant::now()));
    }

    #[test]
    fn green_check_not_due_if_cadence_not_elapsed() {
        let now = Instant::now();
        let s = HealthDecisionState {
            last_successful_interaction: Some(now),
            current_health: HealthState::Green,
        };
        assert!(!s.is_health_check_due(now + Duration::from_secs(60)));
    }

    #[test]
    fn green_check_due_after_cadence() {
        let now = Instant::now();
        let s = HealthDecisionState {
            last_successful_interaction: Some(now),
            current_health: HealthState::Green,
        };
        assert!(s.is_health_check_due(now + GREEN_CADENCE));
    }

    #[test]
    fn degraded_check_due_after_fifteen_seconds() {
        let now = Instant::now();
        let s = HealthDecisionState {
            last_successful_interaction: Some(now),
            current_health: HealthState::Red,
        };
        assert!(s.is_health_check_due(now + Duration::from_secs(15)));
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
            Ok(())
        }
        async fn post_stats(&self, _: &QueuedItem) -> Result<(), PortalCallError> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn verify_now_triggers_immediate_health_check() {
        let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: count.clone(),
        };
        let handle = spawn(io);

        handle
            .command_tx
            .send(PortalCommand::VerifyNow)
            .await
            .unwrap();

        // Let the task dequeue the command (yielding so its select loop
        // gets a chance to run on this single-threaded runtime).
        tokio::task::yield_now().await;
        // Advance past the 2-second sleep so the select's timer arm fires.
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        // Advance again to give the task another chance to drain verify_token.
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;

        assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 1);

        handle
            .command_tx
            .send(PortalCommand::Shutdown)
            .await
            .unwrap();
    }
}
