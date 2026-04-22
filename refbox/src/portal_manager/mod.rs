//! Portal Manager — tracks UWH Portal submission health, retries failures
//! from an on-disk queue, and surfaces problems to the operator.
//!
//! See `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`
//! and `docs/decisions/011-portal-health-indicator.md`.

// Scaffolding: types are defined up front and progressively wired up in Tasks
// 3–14 of the portal health indicator plan. This attribute is removed in
// Task 22 once all types have live callers.
#![allow(dead_code)]

pub mod health;
pub mod queue;

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::sync::mpsc;

use crate::portal_manager::queue::{QueueFile, QueuedItem};

/// Placeholder `PortalTaskIo` used in tests and in the degraded-fallback
/// startup path. This type is intentionally do-nothing — the background
/// retry task spawned by `PortalManager::new` calls its methods but they
/// always succeed without contacting any server.
pub(crate) struct NullIo;

#[async_trait::async_trait]
impl health::PortalTaskIo for NullIo {
    async fn verify_token(&self) -> Result<(), health::PortalCallError> {
        Ok(())
    }
    async fn post_scores(&self, _: &QueuedItem) -> Result<(), health::PortalCallError> {
        Ok(())
    }
    async fn post_stats(&self, _: &QueuedItem) -> Result<(), health::PortalCallError> {
        Ok(())
    }
}

/// Shared handle to the currently-selected event id. Written by the UI
/// side of the app when the operator picks (or clears) an event; read by
/// the background retry task via `UwhPortalIo::verify_token`.
///
/// Wrapped in `Arc<std::sync::Mutex<...>>` (not `tokio::sync::...`)
/// because the `update()` entry point on iced's `RefBoxApp` is a
/// synchronous function that runs on a tokio reactor thread —
/// `tokio::sync::Mutex::blocking_lock` would panic there. We never hold
/// the guard across an `.await`, so a plain `std::sync::Mutex` is
/// correct.
///
/// The UI mirrors `RefBoxApp::current_event_id` into this shared handle
/// on every write, so the background task always sees the latest value
/// without holding the UI lock across a network round-trip.
pub type SelectedEventId =
    std::sync::Arc<std::sync::Mutex<Option<uwh_common::uwhportal::schedule::EventId>>>;

/// Shared handle to the `UwhPortalClient`. Used by both the UI thread
/// (for one-shot requests such as login, schedule fetches, and score
/// posts during the normal flow) and the background retry task (for
/// queued-item retries and token health checks).
///
/// Wrapped in `Arc<std::sync::Mutex<...>>` because `UwhPortalClient`'s
/// methods are infallible-sync request-builders that return a `+ use<>`
/// future — we only hold the guard long enough to construct the request
/// and drop it before awaiting the network round-trip. `std::sync` (not
/// `tokio::sync`) so the UI thread's synchronous read paths (`id()`,
/// `has_token()`, `set_token()`) don't need an async context.
pub type SharedUwhPortalClient =
    std::sync::Arc<std::sync::Mutex<uwh_common::uwhportal::UwhPortalClient>>;

/// Production `PortalTaskIo` backed by a real `UwhPortalClient`.
///
/// Shares the same `UwhPortalClient` handle as the main app via
/// `Arc<Mutex<_>>` so that operator-driven token mutations
/// (set_token / clear_token) are immediately visible to the background
/// retry task without having to restart anything. The lock is only held
/// across the short synchronous portion of each portal call (URL building
/// and request construction); the returned future is `+ use<>` on
/// `UwhPortalClient`'s methods, so we drop the guard before awaiting the
/// network round-trip.
///
/// The `event_id` is shared mutable state rather than a plain `EventId`
/// because the operator chooses the event after startup. When the event
/// id is `None`, `verify_token` is a no-op that reports success —
/// there's nothing to validate against yet, and we don't want to flash
/// the indicator red before the operator has set up the tournament.
/// `post_scores` / `post_stats` ignore this field; they use the queued
/// item's own event id.
pub struct UwhPortalIo {
    client: SharedUwhPortalClient,
    event_id: SelectedEventId,
}

impl UwhPortalIo {
    pub fn new(client: SharedUwhPortalClient, event_id: SelectedEventId) -> Self {
        Self { client, event_id }
    }
}

/// Collapse a portal-client error into `PortalCallError::Failed`. The
/// uwh-common API does not distinguish token-expiry, conflict, server
/// error, or network failure — they all come back as `Box<dyn Error>` —
/// so the background task treats every failure the same way (retry later
/// on its cadence). See ADR 011 amendment (2026-04-21).
fn classify_error<E: std::fmt::Display>(e: E) -> health::PortalCallError {
    health::PortalCallError::Failed(e.to_string())
}

/// Parse an `event_id` string (from a `QueuedItem`) into an `EventId`.
/// Items entered the queue via `enqueue_game_end`, which accepts any
/// string, so we defensively map parse failures to a portal call error
/// instead of panicking. In practice the queued string is the full form
/// produced by `EventId::full()`, so this will almost always succeed.
fn parse_event_id(
    raw: &str,
) -> Result<uwh_common::uwhportal::schedule::EventId, health::PortalCallError> {
    uwh_common::uwhportal::schedule::EventId::from_full(raw).map_err(|e| {
        health::PortalCallError::Failed(format!("invalid queued event_id {raw:?}: {e}"))
    })
}

/// Take a snapshot of the currently-selected event id for use by the
/// background task. The guard is dropped before the function returns, so
/// the caller can safely hold the returned `Option<EventId>` across an
/// `.await`. The mutex is poisoned only if some other thread panicked
/// while holding it; if that happens we treat it the same as "no event
/// selected" so the retry task keeps running harmlessly.
fn snapshot_event_id(shared: &SelectedEventId) -> Option<uwh_common::uwhportal::schedule::EventId> {
    // why this cannot panic: the guarded data is a plain `Option` and no
    // writer panics while holding it; poisoning simply yields the last
    // value, which we clone out.
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[async_trait::async_trait]
impl health::PortalTaskIo for UwhPortalIo {
    async fn verify_token(&self) -> Result<(), health::PortalCallError> {
        let event_id = match snapshot_event_id(&self.event_id) {
            Some(id) => id,
            // No event selected yet — nothing to verify. Reporting success
            // keeps the indicator green; reporting failure would flash red
            // on a freshly-started refbox before the operator has even
            // picked a tournament.
            None => return Ok(()),
        };

        let fut = {
            // why this cannot panic: the guarded data (`UwhPortalClient`)
            // is only mutated via `set_token`/`clear_token`, which do not
            // panic, so the mutex never gets poisoned in practice; we use
            // `unwrap_or_else(into_inner)` defensively so even a poisoned
            // mutex keeps the background task alive.
            let guard = self
                .client
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.verify_token(&event_id)
            // guard drops here so token mutations from the UI thread can
            // land while the network call is in flight.
        };
        fut.await.map_err(classify_error)
    }

    async fn post_scores(&self, item: &QueuedItem) -> Result<(), health::PortalCallError> {
        let event_id = parse_event_id(&item.id.event_id)?;
        let scores = uwh_common::bundles::BlackWhiteBundle {
            black: item.black_score,
            white: item.white_score,
        };
        let game_number = item.id.game_number.clone();
        let force = item.force;
        let fut = {
            // why this cannot panic: see `verify_token` above.
            let guard = self
                .client
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.post_game_scores(&event_id, &game_number, scores, force)
        };
        fut.await.map_err(classify_error)
    }

    async fn post_stats(&self, item: &QueuedItem) -> Result<(), health::PortalCallError> {
        let event_id = parse_event_id(&item.id.event_id)?;
        let game_number = item.id.game_number.clone();
        let stats = item.stats.clone();
        let fut = {
            // why this cannot panic: see `verify_token` above.
            let guard = self
                .client
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.post_game_stats(&event_id, &game_number, stats)
        };
        fut.await.map_err(classify_error)
    }
}

/// Overall health state of the portal connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Last exchange succeeded; queue is empty; token is valid.
    Green,
    /// A check is in flight or the last call was slow-but-successful.
    Yellow,
    /// At least one item needs attention.
    Red,
}

/// Overlay icon currently showing on top of the status dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayState {
    /// Plain dot, no overlay.
    None,
    /// Green checkmark shown until `deadline` (instant).
    /// The view layer compares against `Instant::now()` to decide visibility.
    RecentSuccess,
    /// Red exclamation mark (persists while any item needs attention).
    AttentionNeeded,
}

/// Combined state consumed by the time-banner helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalIndicatorState {
    pub health: HealthState,
    pub overlay: OverlayState,
}

impl Default for PortalIndicatorState {
    fn default() -> Self {
        Self {
            health: HealthState::Green,
            overlay: OverlayState::None,
        }
    }
}

/// Unique identifier for a queued item (event_id + game_number).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId {
    pub event_id: String,
    pub game_number: String,
}

/// Event emitted by the portal manager's background task for the iced
/// Subscription to convert into a `Message`.
#[derive(Debug, Clone)]
pub enum PortalEvent {
    HealthChanged(HealthState),
    OverlayChanged(OverlayState),
    ItemAdded(ItemId),
    ItemResolved(ItemId),
    ItemUpdated(ItemId),
}

/// How long the green-checkmark overlay remains visible after a successful submit.
pub const RECENT_SUCCESS_DURATION: Duration = Duration::from_secs(10);

/// How long an item may sit in the queue before it escalates from
/// Yellow (silently retrying) to Red (operator needs to decide).
/// See ADR 011 amendment (2026-04-21).
pub const STUCK_THRESHOLD: TimeDuration = TimeDuration::minutes(30);

pub fn is_item_stuck(item: &QueuedItem, now: OffsetDateTime) -> bool {
    (now - item.queued_at) >= STUCK_THRESHOLD
}

pub struct PortalManager {
    queue: QueueFile,
    check_in_flight: bool,
    /// Set true when the last `verify_token` probe failed. Cleared on
    /// the next successful probe or by `token_refreshed()` after the
    /// operator re-logs-in.
    token_known_problem: bool,
    recent_success_until: Option<Instant>,
    indicator_state: PortalIndicatorState,
    command_tx: mpsc::Sender<health::PortalCommand>,
    config_dir: std::path::PathBuf,
}

impl PortalManager {
    /// Test-only constructor. The production constructor is introduced in Task 6.
    #[cfg(test)]
    pub(crate) fn new_for_test(
        queue: QueueFile,
        check_in_flight: bool,
        token_known_problem: bool,
    ) -> Self {
        let (tx, _rx) = mpsc::channel(1);
        let mut m = Self {
            queue,
            check_in_flight,
            token_known_problem,
            recent_success_until: None,
            indicator_state: PortalIndicatorState::default(),
            command_tx: tx,
            config_dir: std::env::temp_dir(),
        };
        m.recompute_indicator();
        m
    }

    pub fn indicator_state(&self) -> PortalIndicatorState {
        self.indicator_state
    }

    pub fn mark_recent_success(&mut self) {
        self.recent_success_until = Some(Instant::now() + RECENT_SUCCESS_DURATION);
        self.recompute_indicator();
    }

    fn has_stuck_items(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        self.queue.items.iter().any(|it| is_item_stuck(it, now))
    }

    fn has_any_queue_items(&self) -> bool {
        !self.queue.items.is_empty()
    }

    fn needs_attention(&self) -> bool {
        self.token_known_problem || self.has_stuck_items()
    }

    fn recompute_indicator(&mut self) {
        let health = if self.needs_attention() {
            HealthState::Red
        } else if self.check_in_flight || self.has_any_queue_items() {
            HealthState::Yellow
        } else {
            HealthState::Green
        };

        let overlay = if self.needs_attention() {
            OverlayState::AttentionNeeded
        } else if self
            .recent_success_until
            .map(|t| Instant::now() < t)
            .unwrap_or(false)
        {
            OverlayState::RecentSuccess
        } else {
            OverlayState::None
        };

        self.indicator_state = PortalIndicatorState { health, overlay };
    }

    fn find_mut(&mut self, id: &ItemId) -> Option<&mut QueuedItem> {
        self.queue.items.iter_mut().find(|it| it.id == *id)
    }
}

impl PortalManager {
    /// Construct a new PortalManager. Loads the queue from `config_dir`
    /// (starting fresh if the file is missing or corrupted), spawns the
    /// background retry task driven by the given `PortalTaskIo`, and
    /// returns the receiver side of the event channel so the caller can
    /// feed it into an iced Subscription.
    pub fn new(
        config_dir: &std::path::Path,
        io: impl health::PortalTaskIo + Send + Sync + 'static,
    ) -> std::io::Result<(Self, mpsc::Receiver<PortalEvent>)> {
        let queue = queue::load_or_empty(config_dir)?;
        let handle = health::spawn(io);
        let command_tx = handle.command_tx.clone();

        let mut m = Self {
            queue,
            check_in_flight: false,
            token_known_problem: false,
            recent_success_until: None,
            indicator_state: PortalIndicatorState::default(),
            command_tx,
            config_dir: config_dir.to_path_buf(),
        };
        m.recompute_indicator();
        m.push_queue_snapshot();
        Ok((m, handle.event_rx))
    }

    /// Constructs a `PortalManager` that does not attempt any disk or
    /// network I/O. Used as a last-resort fallback when both the user
    /// config dir and the system temp dir reject I/O — the refbox's
    /// core game functions still work, and the portal indicator shows
    /// Red so the operator sees there's a problem.
    ///
    /// No background task is spawned: there's nothing for it to do,
    /// and spawning one with `NullIo` would cause `verify_token` to
    /// succeed and clear the red state, hiding the problem.
    ///
    /// The returned receiver is a dummy that never emits events.
    pub(crate) fn new_degraded() -> (Self, mpsc::Receiver<PortalEvent>) {
        // Build (sender, receiver) pairs where the senders go nowhere:
        // the event-channel sender is discarded so the returned receiver
        // never emits, and the command-channel sender is kept on the
        // manager only because its type demands it — no background task
        // exists to receive from it.
        let (_, rx) = mpsc::channel(1);
        let (command_tx, _command_rx) = mpsc::channel(1);

        let mut m = Self {
            queue: QueueFile::empty(),
            check_in_flight: false,
            // Key: indicator will show Red so the operator sees the problem.
            token_known_problem: true,
            recent_success_until: None,
            indicator_state: PortalIndicatorState::default(),
            command_tx,
            config_dir: std::env::temp_dir(),
        };
        m.recompute_indicator();
        (m, rx)
    }

    /// Send the current queue snapshot to the background task. Called
    /// after every queue mutation so the task's view stays fresh.
    fn push_queue_snapshot(&self) {
        let tx = self.command_tx.clone();
        let snap = self.queue.clone();
        tokio::spawn(async move {
            let _ = tx.send(health::PortalCommand::QueueUpdated(snap)).await;
        });
    }

    /// Enqueue a game-end submission and trigger an immediate attempt.
    /// Writes to disk *before* attempting the submit so a crash between
    /// write and send does not lose the score.
    pub fn enqueue_game_end(
        &mut self,
        event_id: String,
        game_number: String,
        black_score: u8,
        white_score: u8,
        stats: String,
    ) -> std::io::Result<()> {
        let item = QueuedItem {
            id: ItemId {
                event_id,
                game_number,
            },
            black_score,
            white_score,
            stats,
            queued_at: OffsetDateTime::now_utc(),
            attempts: 0,
            last_attempt_at: None,
            force: false,
        };
        self.queue.items.push(item);
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        self.push_queue_snapshot();
        Ok(())
    }

    /// Operator tapped FORCE THIS GAME RESULT on the attention action page.
    /// Sets the item's `force` flag so the next submit sends `force=true`,
    /// resets the attempt counter and the last-attempt timestamp so the
    /// background task retries immediately, and resets `queued_at` to
    /// "now" so the item is no longer considered stuck (the operator
    /// has restarted its 30-minute clock by making a decision).
    pub fn force_submit(&mut self, id: &ItemId) -> std::io::Result<()> {
        if let Some(item) = self.find_mut(id) {
            item.force = true;
            item.attempts = 0;
            item.last_attempt_at = None;
            item.queued_at = OffsetDateTime::now_utc();
            queue::save(&self.config_dir, &self.queue)?;
        }
        self.recompute_indicator();
        self.push_queue_snapshot();
        Ok(())
    }

    /// Operator tapped DISCARD THIS SUBMISSION on the attention action page.
    /// Removes the item from the queue without submitting. Whatever the
    /// portal currently has for that game stands.
    pub fn discard(&mut self, id: &ItemId) -> std::io::Result<()> {
        self.queue.items.retain(|it| it.id != *id);
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        self.push_queue_snapshot();
        Ok(())
    }

    /// Called after a successful portal re-login. Clears the global
    /// token-problem flag and resets every queued item's attempt
    /// counter and last-attempt timestamp so the background task
    /// retries them immediately on its next tick.
    pub fn token_refreshed(&mut self) -> std::io::Result<()> {
        self.token_known_problem = false;
        for item in &mut self.queue.items {
            item.attempts = 0;
            item.last_attempt_at = None;
        }
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        self.push_queue_snapshot();
        Ok(())
    }

    /// Force an immediate health check (fires verify_token out-of-band).
    /// Sends a `VerifyNow` command to the background task; the task will
    /// clear its last-success marker so the next tick treats the token
    /// as cadence-due and re-verifies.
    pub fn verify_now(&mut self) {
        let tx = self.command_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(health::PortalCommand::VerifyNow).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal_manager::queue::{QueueFile, QueuedItem};
    use time::{Duration as TimeDuration, OffsetDateTime};

    fn mk_young_item() -> QueuedItem {
        QueuedItem {
            id: ItemId {
                event_id: "e".into(),
                game_number: "G1".into(),
            },
            black_score: 0,
            white_score: 0,
            stats: "{}".into(),
            queued_at: OffsetDateTime::now_utc(),
            attempts: 0,
            last_attempt_at: None,
            force: false,
        }
    }

    fn mk_stuck_item() -> QueuedItem {
        let mut it = mk_young_item();
        it.queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(31);
        it
    }

    #[test]
    fn empty_queue_and_no_problems_is_green() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        assert_eq!(m.indicator_state().health, HealthState::Green);
        assert_eq!(m.indicator_state().overlay, OverlayState::None);
    }

    #[test]
    fn young_pending_item_is_yellow_no_overlay() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_young_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
        assert_eq!(m.indicator_state().overlay, OverlayState::None);
    }

    #[test]
    fn stuck_item_is_red_with_attention_overlay() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_stuck_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert_eq!(m.indicator_state().overlay, OverlayState::AttentionNeeded);
    }

    #[test]
    fn token_known_problem_is_red_with_attention_overlay() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, true);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert_eq!(m.indicator_state().overlay, OverlayState::AttentionNeeded);
    }

    #[test]
    fn health_check_in_flight_is_yellow() {
        let m = PortalManager::new_for_test(QueueFile::empty(), true, false);
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
    }

    #[test]
    fn recent_success_overlay_is_suppressed_by_stuck_item() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_stuck_item()],
        };
        let mut m = PortalManager::new_for_test(q, false, false);
        m.mark_recent_success();
        assert_eq!(m.indicator_state().overlay, OverlayState::AttentionNeeded);
    }

    #[test]
    fn recent_success_shows_when_queue_empty() {
        let mut m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        m.mark_recent_success();
        assert_eq!(m.indicator_state().overlay, OverlayState::RecentSuccess);
    }

    #[tokio::test]
    async fn enqueue_game_end_appends_item_and_turns_yellow() {
        // Use a temp dir so the save succeeds.
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        assert_eq!(m.indicator_state().health, HealthState::Green);

        m.enqueue_game_end("event".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();

        // Fresh item: Yellow (retrying silently), not Red.
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
        assert_eq!(m.queue.items.len(), 1);
    }

    #[tokio::test]
    async fn discard_removes_item_and_returns_to_green() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();

        let id = m.queue.items[0].id.clone();
        m.discard(&id).unwrap();

        assert_eq!(m.indicator_state().health, HealthState::Green);
        assert!(m.queue.items.is_empty());
    }

    #[tokio::test]
    async fn force_submit_flags_force_and_resets_attempt_counters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();

        // Pretend the item has been retrying for a while.
        m.queue.items[0].attempts = 7;
        m.queue.items[0].last_attempt_at = Some(OffsetDateTime::now_utc());

        m.force_submit(&id).unwrap();

        assert!(m.queue.items[0].force);
        assert_eq!(m.queue.items[0].attempts, 0);
        assert!(m.queue.items[0].last_attempt_at.is_none());
    }

    #[tokio::test]
    async fn token_refreshed_clears_flag_and_resets_queue_items() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        m.token_known_problem = true;
        m.queue.items[0].attempts = 4;
        m.queue.items[0].last_attempt_at = Some(OffsetDateTime::now_utc());

        m.token_refreshed().unwrap();

        assert!(!m.token_known_problem);
        assert_eq!(m.queue.items[0].attempts, 0);
        assert!(m.queue.items[0].last_attempt_at.is_none());
    }

    #[test]
    fn new_degraded_indicator_has_token_known_problem_and_no_spawned_task() {
        let (manager, mut rx) = PortalManager::new_degraded();

        // The indicator should reflect token_known_problem (Red state).
        let state = manager.indicator_state();
        assert_eq!(
            state.health,
            HealthState::Red,
            "degraded mode must surface the failure to the operator via a red dot"
        );

        // The queue should be empty (no persistence attempted).
        assert_eq!(manager.queue.items.len(), 0);

        // The returned receiver should not receive any events (no spawned task,
        // and the sender half was dropped at construction).
        // try_recv should return an Err (either Empty or Disconnected).
        assert!(
            rx.try_recv().is_err(),
            "degraded mode must not produce portal events"
        );
    }

    #[test]
    fn new_degraded_does_not_touch_disk() {
        // Smoke test: constructing a degraded manager must not perform
        // any filesystem I/O. We can't easily intercept I/O, but we can
        // verify the config_dir field points at temp_dir (a safe
        // non-persistent default) rather than any user-supplied path.
        let (manager, _rx) = PortalManager::new_degraded();
        assert_eq!(manager.config_dir, std::env::temp_dir());
    }
}
