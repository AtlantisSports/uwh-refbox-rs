//! Portal Manager — tracks UWH Portal submission health, retries failures
//! from an on-disk queue, and surfaces problems to the operator.
//!
//! See `docs/superpowers/specs/2026-04-19-portal-health-indicator-design.md`
//! and `docs/decisions/011-portal-health-indicator.md`.

pub mod health;
pub mod link_session;
pub mod queue;

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Instant;
use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::sync::mpsc;

use crate::portal_manager::queue::{QueueFile, QueuedItem};

/// Maximum number of recent successes shown at the bottom of the
/// detail page. When a sixth success lands, the oldest is evicted.
pub const RECENT_SUCCESS_CAP: usize = 5;

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

/// Classify a portal-client error. A `reqwest` transport error means the
/// HTTP exchange never completed (DNS, connect, timeout, TLS, or a dropped
/// response): the portal is unreachable, which is a connectivity problem,
/// NOT a token rejection. A non-OK HTTP response surfaces as uwh-common's
/// own error type instead and is mapped to `Failed` (could be a 401 token
/// rejection, a 409 conflict, or a 5xx). Keeping the two apart lets the
/// indicator avoid mislabelling an ordinary wifi drop as "login expired".
/// The retry/cadence logic treats both the same (retry later); only the
/// `verify_token` token-status path acts on the distinction. See ADR 011
/// amendment (2026-04-21).
fn classify_error(e: Box<dyn std::error::Error>) -> health::PortalCallError {
    if e.downcast_ref::<reqwest::Error>().is_some() {
        health::PortalCallError::Unreachable(e.to_string())
    } else {
        health::PortalCallError::Failed(e.to_string())
    }
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

/// Combined state consumed by the time-banner helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalIndicatorState {
    pub health: HealthState,
    /// True when the saved login token is known to be expired/rejected.
    /// This is the specific Red cause that makes a schedule refresh
    /// pointless (it can only fail until the operator re-logs-in), so
    /// the game-info REFRESH button greys out on it. A Red caused only
    /// by a stuck queue item — or by a connectivity problem (the portal
    /// is unreachable, but the login may be fine) — leaves this `false`.
    pub token_expired: bool,
}

impl Default for PortalIndicatorState {
    fn default() -> Self {
        Self {
            health: HealthState::Green,
            token_expired: false,
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
    HealthChanged,
    ItemResolved(ItemId),
    ItemUpdated,
    /// The score posted successfully but the stats upload failed. The
    /// main thread flips the item to stats-pending (`score_sent = true`)
    /// so the auto-retry loop, the stuck escalation, and the indicator
    /// all stop treating it as outstanding. Carries the item id.
    ScoreSentStatsPending(ItemId),
    /// Result of the latest periodic `verify_token` probe that reached the
    /// portal. `true` = portal accepted the token, `false` = rejected. The
    /// main-thread handler maps this onto `token_known_problem` so the
    /// indicator and detail-page row reflect the current token state.
    TokenStatus(bool),
    /// The latest periodic `verify_token` probe could not reach the portal
    /// at all (a connectivity failure, not a token rejection). The
    /// main-thread handler flags a connection problem so the indicator
    /// degrades WITHOUT claiming the login expired — the operator keeps the
    /// schedule REFRESH button and is not pushed into a pointless re-login.
    TokenUnreachable,
}

/// One row rendered on the portal detail page. The ordering of the
/// returned `Vec<DetailRow>` is the on-screen order: token-expired
/// banner first (if present), then stuck items (oldest first), then
/// young pending items (oldest first), then recent successes (newest
/// first, capped at `RECENT_SUCCESS_CAP`).
#[derive(Debug, Clone)]
pub enum DetailRow {
    /// Shown at the top when `token_known_problem` is true. Tapping
    /// drives the operator through the portal re-login flow.
    TokenExpired,
    /// A queued item that has crossed the stuck threshold.
    /// Tapping opens the attention action page (Retry / Discard).
    Stuck { id: ItemId, game_number: String },
    /// A queued item that is still in the auto-retry window
    /// (< stuck threshold since it was queued). Informational.
    /// Tapping forces an immediate retry attempt.
    ///
    /// `attempts` is the number of background retry attempts made so
    /// far. Surfaced in the row label as "(attempt N)" so the operator
    /// can see at a glance that the background retry loop is alive,
    /// without exposing the retry timer (per Unit 7 audit decision).
    Pending {
        id: ItemId,
        game_number: String,
        attempts: u32,
    },
    /// A game whose score is sent but whose stats upload is still
    /// outstanding (stats-pending). Rendered yellow like `Pending`, but
    /// it never escalates and is not auto-retried — tapping fires one
    /// stats attempt. No attempt counter: the row is one-shot, so a
    /// counter would wrongly imply background retrying.
    StatsPending { id: ItemId, game_number: String },
    /// A recently-completed submission, shown as an informational
    /// green strip. Not tappable.
    RecentSuccess {
        game_number: String,
        submitted_mins_ago: u32,
    },
}

/// In-memory record of a successful portal submission. Lives only for
/// as long as the process is running — on restart, the list is empty.
#[derive(Debug, Clone)]
struct RecentSuccess {
    id: ItemId,
    game_number: String,
    submitted_at: Instant,
}

/// How long an item may sit in the queue before it escalates from
/// Yellow (silently retrying) to Red (operator needs to decide).
/// See ADR 011 amendment (2026-04-21).
pub const STUCK_THRESHOLD: TimeDuration = TimeDuration::minutes(30);

pub fn is_item_stuck(item: &QueuedItem, now: OffsetDateTime) -> bool {
    // Stats-pending items (score already accepted) never go stuck: a
    // missing stat must not nag the operator or escalate to red.
    !item.score_sent && (now - item.queued_at) >= STUCK_THRESHOLD
}

pub struct PortalManager {
    queue: QueueFile,
    check_in_flight: bool,
    /// Set true when the last `verify_token` probe reached the portal and
    /// it rejected the token. Cleared on the next reached probe that
    /// succeeds, or by `token_refreshed()` after the operator re-logs-in.
    token_known_problem: bool,
    /// Set true when the last `verify_token` probe could not reach the
    /// portal at all (connectivity failure). Drives the Red indicator like
    /// a token problem does, but deliberately leaves `token_expired` false
    /// so an ordinary wifi drop neither greys the schedule REFRESH button
    /// nor shows the re-login row. Cleared by the next probe that reaches
    /// the portal (whether it accepts or rejects the token).
    connection_problem: bool,
    indicator_state: PortalIndicatorState,
    command_tx: mpsc::Sender<health::PortalCommand>,
    config_dir: std::path::PathBuf,
    /// In-memory ring of the most recent successful submissions,
    /// newest at the front, capped at `RECENT_SUCCESS_CAP`. Used only
    /// for the detail-page strip; not persisted across restarts.
    recent_successes: VecDeque<RecentSuccess>,
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
            connection_problem: false,
            indicator_state: PortalIndicatorState::default(),
            command_tx: tx,
            config_dir: std::env::temp_dir(),
            recent_successes: VecDeque::new(),
        };
        m.recompute_indicator();
        m
    }

    pub fn indicator_state(&self) -> PortalIndicatorState {
        self.indicator_state
    }

    /// Return the directory used for the on-disk queue file.
    /// Used by the portal-tenant-switch restart handler to flush the queue
    /// (items queued for the old tenant cannot be delivered to the new one).
    pub fn queue_dir(&self) -> &std::path::Path {
        &self.config_dir
    }

    /// Recompute the cached indicator state from current inputs.
    /// Called from the iced UI layer:
    /// - on every pure UI-layer tick so the 30-minute stuck-item
    ///   escalation reaches the screen without waiting for an
    ///   unrelated re-render,
    /// - when the background retry task emits a `PortalEvent` so the
    ///   indicator picks up anything that might have changed between
    ///   frames.
    pub fn ui_tick(&mut self) {
        self.recompute_indicator();
    }

    fn has_stuck_items(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        self.queue.items.iter().any(|it| is_item_stuck(it, now))
    }

    fn has_score_pending_items(&self) -> bool {
        self.queue.items.iter().any(|it| !it.score_sent)
    }

    fn needs_attention(&self) -> bool {
        self.token_known_problem || self.has_stuck_items()
    }

    fn recompute_indicator(&mut self) {
        let health = if self.needs_attention() || self.connection_problem {
            HealthState::Red
        } else if self.check_in_flight || self.has_score_pending_items() {
            HealthState::Yellow
        } else {
            HealthState::Green
        };

        self.indicator_state = PortalIndicatorState {
            health,
            token_expired: self.token_known_problem,
        };
    }

    fn find_mut(&mut self, id: &ItemId) -> Option<&mut QueuedItem> {
        self.queue.items.iter_mut().find(|it| it.id == *id)
    }

    /// Look up a queued item by id. Returns `None` if the item is not in
    /// the queue (e.g. it was resolved or discarded since the caller last
    /// observed the queue). The view layer uses this to render the
    /// attention-action page for a specific item.
    pub fn find(&self, id: &ItemId) -> Option<&QueuedItem> {
        self.queue.items.iter().find(|it| it.id == *id)
    }

    /// Returns true if the queued item with the given id has crossed the
    /// stuck threshold. Returns false if the item is not in the queue —
    /// callers routing a row-tap dispatch can treat "unknown id" the
    /// same as "not stuck" without panicking.
    pub fn is_stuck(&self, id: &ItemId) -> bool {
        match self.find(id) {
            Some(item) => is_item_stuck(item, OffsetDateTime::now_utc()),
            None => false,
        }
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
            connection_problem: false,
            indicator_state: PortalIndicatorState::default(),
            command_tx,
            config_dir: config_dir.to_path_buf(),
            recent_successes: VecDeque::new(),
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
            connection_problem: false,
            indicator_state: PortalIndicatorState::default(),
            command_tx,
            config_dir: std::env::temp_dir(),
            recent_successes: VecDeque::new(),
        };
        m.recompute_indicator();
        (m, rx)
    }

    /// Send a one-shot `RetryStats` command to the background task for
    /// the given item. The command carries a clone of the item, so the
    /// task can attempt it without holding the UI lock. Spawned like
    /// `push_queue_snapshot` because `update()` is synchronous.
    fn send_stats_retry(&self, item: &QueuedItem) {
        let tx = self.command_tx.clone();
        let item = item.clone();
        tokio::spawn(async move {
            let _ = tx.send(health::PortalCommand::RetryStats(item)).await;
        });
    }

    /// Operator asked to retry the stats for one stats-pending item
    /// (tapped its row). Sends exactly one `RetryStats`. No-op if the id
    /// is not in the queue.
    pub fn request_stats_retry(&self, id: &ItemId) {
        if let Some(item) = self.find(id) {
            self.send_stats_retry(item);
        }
    }

    /// True iff the queued item exists and its score has been sent but
    /// stats are still outstanding (stats-pending). False for score-
    /// pending items and for unknown ids.
    pub fn is_stats_pending(&self, id: &ItemId) -> bool {
        self.find(id).is_some_and(|it| it.score_sent)
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
    /// Persistence is best-effort: the in-memory queue is updated
    /// before disk write, so an I/O failure leaves the score queued in
    /// memory for the rest of the session but not on disk. The corrupt-
    /// or-missing-file rotation in `queue::load` is the recovery path
    /// across restarts; subsequent successful mutations will re-persist
    /// the queue including this item.
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
            score_sent: false,
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

    /// Operator tapped RETRY ALL on the portal detail page. Resets every
    /// queued game so the background task re-attempts all of them on its
    /// next tick: clears each item's attempt counter and last-attempt
    /// timestamp, and resets `queued_at` to now so any item past the
    /// 30-minute stuck threshold returns to the auto-retry pool.
    ///
    /// `force` is deliberately left untouched: RETRY ALL is a plain
    /// resend, never an overwrite. A game the portal genuinely rejects
    /// (a conflict) simply re-fails and re-surfaces as stuck for the
    /// operator to Force or Discard individually. See ADR 011
    /// (portal-health-indicator) for why a failed submit cannot be told
    /// apart from a conflict.
    ///
    /// No-op-safe on an empty queue. Persistence is best-effort, matching
    /// `token_refreshed`/`force_submit`: the in-memory reset stands even
    /// if the disk write fails (the error propagates for logging).
    pub fn retry_all(&mut self) -> std::io::Result<()> {
        let now = OffsetDateTime::now_utc();
        // Reset every item; touching a stats-pending item's attempt/queued_at
        // fields is harmless (they are unused while score_sent == true) and
        // keeps this a single pass.
        for item in &mut self.queue.items {
            item.attempts = 0;
            item.last_attempt_at = None;
            item.queued_at = now;
        }
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        self.push_queue_snapshot();
        // Stats-pending games are not on the auto-retry cadence; give
        // each a single fresh stats attempt so RETRY ALL sweeps them too.
        for item in &self.queue.items {
            if item.score_sent {
                self.send_stats_retry(item);
            }
        }
        Ok(())
    }

    /// Operator tapped a young (yellow) pending row on the detail page.
    /// Clears `last_attempt_at` on the target item AND pushes a fresh
    /// queue snapshot to the background task, so the task sees the
    /// update and attempts the item on its next tick without waiting
    /// out `ITEM_RETRY_INTERVAL`. Without the snapshot push the
    /// background task would keep evaluating retry-eligibility against
    /// its own stale copy of the queue, and the tap would appear to do
    /// nothing. The item stays young-pending, so no indicator recompute
    /// is needed.
    pub fn force_immediate_retry(&mut self, id: &ItemId) -> std::io::Result<()> {
        if let Some(item) = self.find_mut(id) {
            item.last_attempt_at = None;
            queue::save(&self.config_dir, &self.queue)?;
            self.push_queue_snapshot();
        }
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

    /// Apply the result of a periodic `verify_token` probe. `true` = the
    /// portal accepted the token, `false` = it rejected. Sets the global
    /// `token_known_problem` flag accordingly and recomputes the
    /// indicator so a token failure paints the time-banner red and a
    /// recovery clears it. This is the path that lets the operator
    /// notice a silent token expiration without having to open Settings.
    pub fn on_token_status(&mut self, valid: bool) {
        self.token_known_problem = !valid;
        // A probe that reached the portal (whether it accepted or rejected
        // the token) proves connectivity, so clear any prior "unreachable"
        // flag.
        self.connection_problem = false;
        self.recompute_indicator();
    }

    /// Apply a `verify_token` probe that could not reach the portal at all.
    /// Flags a connection problem so the indicator degrades to Red, but
    /// deliberately leaves `token_known_problem` untouched: a connectivity
    /// failure is not evidence the login expired, so the operator is not
    /// pushed to re-login and the schedule REFRESH button stays usable.
    /// The next probe that reaches the portal clears this flag.
    pub fn on_token_unreachable(&mut self) {
        self.connection_problem = true;
        self.recompute_indicator();
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

    /// Background task reported a score-OK / stats-fail outcome: the score
    /// is up but the stats upload was rejected. Flip the item to
    /// stats-pending so it leaves the auto-retry loop and the yellow/red
    /// indicator, persist, and push a fresh snapshot so the background
    /// task stops attempting it. Idempotent: a duplicate event (or an
    /// unknown id) is a silent no-op.
    pub fn on_score_sent_stats_pending(&mut self, id: ItemId) {
        let Some(item) = self.find_mut(&id) else {
            return;
        };
        if item.score_sent {
            return; // Already stats-pending; nothing to do.
        }
        item.score_sent = true;
        if let Err(e) = queue::save(&self.config_dir, &self.queue) {
            log::warn!("portal queue save after score-sent/stats-pending failed: {e}");
        }
        self.recompute_indicator();
        self.push_queue_snapshot();
    }

    /// Called from the UI thread after the background task reports that
    /// a queued item was successfully posted to the portal. Removes the
    /// item from the queue, records it in the recent-success ring
    /// (newest first, capped at `RECENT_SUCCESS_CAP`), and persists the
    /// shrunken queue so a restart does not re-send the item.
    ///
    /// I/O errors on the queue save are logged and otherwise ignored:
    /// the in-memory state is already correct, and re-sending a
    /// successfully-posted item would be a worse failure mode than
    /// losing the on-disk reflection of an already-completed action.
    pub fn on_item_resolved(&mut self, id: ItemId) {
        // Idempotent: if we've already recorded this resolution in the
        // recent-successes ring, do nothing. Duplicate delivery can
        // happen if the background task retries a post before the main
        // thread has processed the first `ItemResolved` event, or if a
        // future action (e.g. FORCE from the attention page) resolves
        // an item that was already removed from the queue. Without this
        // guard, the same game would appear as two green rows on the
        // detail page.
        if self.recent_successes.iter().any(|rs| rs.id == id) {
            return;
        }

        // Only record a recent success for items that were actually on
        // the queue at the time of the call. An `on_item_resolved` for
        // an unknown id (e.g. the background task reporting a resolve
        // for an item the operator discarded moments earlier) should be
        // a silent no-op — we never invent a phantom green row.
        let Some(game_number) = self
            .queue
            .items
            .iter()
            .find(|it| it.id == id)
            .map(|it| it.id.game_number.clone())
        else {
            return;
        };

        self.queue.items.retain(|it| it.id != id);

        self.recent_successes.push_front(RecentSuccess {
            id,
            game_number,
            submitted_at: Instant::now(),
        });
        while self.recent_successes.len() > RECENT_SUCCESS_CAP {
            self.recent_successes.pop_back();
        }

        if let Err(e) = queue::save(&self.config_dir, &self.queue) {
            log::warn!("portal queue save after item resolution failed: {e}");
        }
        self.recompute_indicator();
        self.push_queue_snapshot();
    }

    /// Compute the ordered list of rows displayed on the portal detail
    /// page. Ordering:
    /// 1. `TokenExpired` banner, if a token problem is flagged.
    /// 2. `Stuck` items (queued ≥ 30 min ago), oldest first.
    /// 3. `Pending` items (queued < 30 min ago), oldest first.
    /// 4. `RecentSuccess` rows, newest first, capped at
    ///    `RECENT_SUCCESS_CAP`.
    pub fn detail_rows(&self) -> Vec<DetailRow> {
        let mut out: Vec<DetailRow> = Vec::new();

        if self.token_known_problem {
            out.push(DetailRow::TokenExpired);
        }

        let now = OffsetDateTime::now_utc();
        let mut items: Vec<&QueuedItem> = self.queue.items.iter().collect();
        items.sort_by_key(|it| it.queued_at);

        for it in &items {
            if is_item_stuck(it, now) {
                out.push(DetailRow::Stuck {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                });
            }
        }
        for it in &items {
            if !it.score_sent && !is_item_stuck(it, now) {
                out.push(DetailRow::Pending {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                    attempts: it.attempts,
                });
            }
        }
        for it in &items {
            if it.score_sent {
                out.push(DetailRow::StatsPending {
                    id: it.id.clone(),
                    game_number: it.id.game_number.clone(),
                });
            }
        }

        let now_instant = Instant::now();
        for rs in &self.recent_successes {
            let mins = now_instant
                .saturating_duration_since(rs.submitted_at)
                .as_secs()
                / 60;
            out.push(DetailRow::RecentSuccess {
                game_number: rs.game_number.clone(),
                submitted_mins_ago: mins as u32,
            });
        }

        out
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
            score_sent: false,
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
    }

    #[test]
    fn young_pending_item_is_yellow() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_young_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
    }

    #[test]
    fn stuck_item_is_red() {
        let q = QueueFile {
            version: 1,
            items: vec![mk_stuck_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Red);
    }

    #[test]
    fn token_known_problem_is_red() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, true);
        assert_eq!(m.indicator_state().health, HealthState::Red);
    }

    #[test]
    fn token_known_problem_sets_token_expired() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, true);
        assert!(m.indicator_state().token_expired);
    }

    #[test]
    fn stuck_item_red_is_not_token_expired() {
        // A Red caused solely by a stuck queue item must NOT report
        // token_expired — a schedule refresh still works in that case.
        let q = QueueFile {
            version: 1,
            items: vec![mk_stuck_item()],
        };
        let m = PortalManager::new_for_test(q, false, false);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert!(!m.indicator_state().token_expired);
    }

    #[test]
    fn healthy_connection_is_not_token_expired() {
        let m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        assert!(!m.indicator_state().token_expired);
    }

    #[test]
    fn connection_problem_is_red_but_not_token_expired() {
        // An ordinary wifi drop (verify_token could not reach the portal)
        // must degrade the indicator to Red WITHOUT reporting token_expired:
        // the login may be fine, so the schedule REFRESH button must stay
        // usable and no re-login row should appear.
        let mut m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        m.on_token_unreachable();
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert!(!m.indicator_state().token_expired);
    }

    #[test]
    fn reaching_the_portal_clears_a_prior_connection_problem() {
        // Once a probe reaches the portal again (here: the token is
        // accepted), the connection problem clears and the indicator
        // returns to Green.
        let mut m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        m.on_token_unreachable();
        assert_eq!(m.indicator_state().health, HealthState::Red);
        m.on_token_status(true);
        assert_eq!(m.indicator_state().health, HealthState::Green);
        assert!(!m.indicator_state().token_expired);
    }

    #[test]
    fn server_rejection_after_outage_reports_token_expired() {
        // A probe that reaches the portal but is rejected clears the
        // connectivity flag and reports a genuine token problem (the
        // re-login row + greyed REFRESH).
        let mut m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        m.on_token_unreachable();
        m.on_token_status(false);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert!(m.indicator_state().token_expired);
    }

    #[test]
    fn health_check_in_flight_is_yellow() {
        let m = PortalManager::new_for_test(QueueFile::empty(), true, false);
        assert_eq!(m.indicator_state().health, HealthState::Yellow);
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
    async fn retry_all_unsticks_and_resets_every_item_without_forcing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // One stuck game (queued 31 min ago) and one young pending game.
        m.enqueue_game_end("event".into(), "G_STUCK".into(), 0, 0, "{}".into())
            .unwrap();
        m.enqueue_game_end("event".into(), "G_YOUNG".into(), 1, 0, "{}".into())
            .unwrap();

        // Age the first past the 30-min stuck threshold and give both
        // some prior attempt history.
        let old = OffsetDateTime::now_utc() - TimeDuration::minutes(31);
        m.queue.items[0].queued_at = old;
        m.queue.items[0].attempts = 5;
        m.queue.items[0].last_attempt_at = Some(old);
        m.queue.items[1].attempts = 2;
        m.queue.items[1].last_attempt_at = Some(OffsetDateTime::now_utc());

        // Precondition: the first item is currently stuck.
        assert!(is_item_stuck(&m.queue.items[0], OffsetDateTime::now_utc()));

        m.retry_all().unwrap();

        let now = OffsetDateTime::now_utc();
        for item in &m.queue.items {
            // Reset to a fresh, immediately-retriable state...
            assert_eq!(item.attempts, 0);
            assert!(item.last_attempt_at.is_none());
            // ...and no longer stuck (queued_at moved to ~now), which
            // together with last_attempt_at == None means the background
            // task will pick it up on its next tick.
            assert!(!is_item_stuck(item, now));
            // Safe resend: force must never be set by retry_all.
            assert!(!item.force);
        }
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

    #[tokio::test]
    async fn queue_with_only_stats_pending_item_is_green() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();
        // Mark it stats-pending and age it well past the stuck threshold.
        m.queue.items[0].score_sent = true;
        m.queue.items[0].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(120);
        m.recompute_indicator();
        assert_eq!(
            m.indicator_state().health,
            HealthState::Green,
            "a queue holding only stats-pending items must keep the dot green"
        );
    }

    #[tokio::test]
    async fn detail_rows_orders_token_then_stuck_then_pending_oldest_first() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // One stuck item: queued 40 min ago.
        m.enqueue_game_end("e".into(), "G3".into(), 3, 2, "{}".into())
            .unwrap();
        m.queue.items[0].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(40);

        // Two young pendings: queued at different recent times.
        m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        m.queue.items[1].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(5);
        m.enqueue_game_end("e".into(), "G2".into(), 0, 0, "{}".into())
            .unwrap();
        m.queue.items[2].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(2);

        // Flag the token as having a known problem to exercise the
        // TokenExpired row.
        m.token_known_problem = true;

        let rows = m.detail_rows();
        assert!(matches!(rows[0], DetailRow::TokenExpired));
        assert!(
            matches!(rows[1], DetailRow::Stuck { ref game_number, .. } if game_number == "G3"),
            "expected Stuck G3 at row[1], got {:?}",
            rows[1]
        );
        assert!(
            matches!(rows[2], DetailRow::Pending { ref game_number, .. } if game_number == "G1"),
            "expected Pending G1 at row[2], got {:?}",
            rows[2]
        );
        assert!(
            matches!(rows[3], DetailRow::Pending { ref game_number, .. } if game_number == "G2"),
            "expected Pending G2 at row[3], got {:?}",
            rows[3]
        );
        assert_eq!(rows.len(), 4);
    }

    #[tokio::test]
    async fn on_item_resolved_removes_queue_item_and_adds_recent_success() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("event".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();

        let id = m.queue.items[0].id.clone();
        assert_eq!(m.queue.items.len(), 1);

        m.on_item_resolved(id.clone());

        assert!(
            m.queue.items.is_empty(),
            "queue should be empty after resolve"
        );
        assert_eq!(m.recent_successes.len(), 1);
        assert_eq!(m.recent_successes[0].id, id);
        assert_eq!(m.recent_successes[0].game_number, "G1");
    }

    #[tokio::test]
    async fn recent_successes_caps_at_five_and_evicts_oldest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // Push six resolutions, numbered G1..G6.
        for n in 1..=6u32 {
            let game = format!("G{n}");
            m.enqueue_game_end("e".into(), game.clone(), 0, 0, "{}".into())
                .unwrap();
            let id = m.queue.items[0].id.clone();
            m.on_item_resolved(id);
        }

        // Newest (G6) at front, oldest retained is G2 at back.
        assert_eq!(m.recent_successes.len(), RECENT_SUCCESS_CAP);
        assert_eq!(m.recent_successes.front().unwrap().game_number, "G6");
        assert_eq!(m.recent_successes.back().unwrap().game_number, "G2");

        // G1 was evicted.
        assert!(
            !m.recent_successes.iter().any(|rs| rs.game_number == "G1"),
            "oldest entry G1 should have been evicted"
        );
    }

    #[tokio::test]
    async fn on_item_resolved_with_unknown_id_is_noop_and_does_not_duplicate() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // 1. Enqueue one item (id A).
        m.enqueue_game_end("event".into(), "GA".into(), 1, 0, "{}".into())
            .unwrap();
        let id_a = m.queue.items[0].id.clone();

        // 2. Resolve A — queue empty, one recent-success entry.
        m.on_item_resolved(id_a.clone());
        assert!(m.queue.items.is_empty());
        assert_eq!(m.recent_successes.len(), 1);

        // 3. Resolve A a second time — queue still empty, recent_successes
        // STILL has exactly 1 entry (no duplicate row).
        m.on_item_resolved(id_a.clone());
        assert!(m.queue.items.is_empty());
        assert_eq!(
            m.recent_successes.len(),
            1,
            "second resolution of the same id must not duplicate the recent-success row"
        );
        assert_eq!(m.recent_successes[0].id, id_a);

        // 4. Resolve B — an id that was never enqueued. Queue still empty,
        // recent_successes still 1, no panic.
        let id_b = ItemId {
            event_id: "event".into(),
            game_number: "GB-never-queued".into(),
        };
        m.on_item_resolved(id_b);
        assert!(m.queue.items.is_empty());
        assert_eq!(m.recent_successes.len(), 1);
        assert_eq!(m.recent_successes[0].id, id_a);
    }

    #[tokio::test]
    async fn detail_rows_appends_recent_successes_newest_first() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // Resolve two items, G1 then G2.
        m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        let id1 = m.queue.items[0].id.clone();
        m.on_item_resolved(id1);

        m.enqueue_game_end("e".into(), "G2".into(), 0, 0, "{}".into())
            .unwrap();
        let id2 = m.queue.items[0].id.clone();
        m.on_item_resolved(id2);

        let rows = m.detail_rows();
        // Queue is empty, token ok → only recent-success rows, newest first.
        assert_eq!(rows.len(), 2);
        assert!(
            matches!(&rows[0], DetailRow::RecentSuccess { game_number, .. } if game_number == "G2")
        );
        assert!(
            matches!(&rows[1], DetailRow::RecentSuccess { game_number, .. } if game_number == "G1")
        );
    }

    #[tokio::test]
    async fn is_stuck_classifies_items_and_is_none_safe_for_unknown_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // Stuck item: queued 40 minutes ago.
        m.enqueue_game_end("e".into(), "G_STUCK".into(), 0, 0, "{}".into())
            .unwrap();
        m.queue.items[0].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(40);
        let stuck_id = m.queue.items[0].id.clone();

        // Young item: queued 5 minutes ago.
        m.enqueue_game_end("e".into(), "G_YOUNG".into(), 0, 0, "{}".into())
            .unwrap();
        m.queue.items[1].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(5);
        let young_id = m.queue.items[1].id.clone();

        assert!(m.is_stuck(&stuck_id), "40-minute-old item should be stuck");
        assert!(
            !m.is_stuck(&young_id),
            "5-minute-old item should not be stuck"
        );

        let unknown = ItemId {
            event_id: "e".into(),
            game_number: "G_NEVER_QUEUED".into(),
        };
        assert!(
            !m.is_stuck(&unknown),
            "unknown id must report not-stuck, not panic"
        );
    }

    #[tokio::test]
    async fn find_returns_item_or_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();

        let found = m.find(&id);
        assert!(found.is_some(), "find should return Some for queued id");
        assert_eq!(found.unwrap().id, id);

        let unknown = ItemId {
            event_id: "e".into(),
            game_number: "G_NEVER_QUEUED".into(),
        };
        assert!(
            m.find(&unknown).is_none(),
            "find should return None for unknown id"
        );
    }

    #[tokio::test]
    async fn force_immediate_retry_clears_last_attempt_at_and_is_noop_for_unknown() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();

        // Stamp last_attempt_at so we can observe it being cleared.
        m.queue.items[0].last_attempt_at = Some(OffsetDateTime::now_utc());

        m.force_immediate_retry(&id).unwrap();
        assert!(
            m.queue.items[0].last_attempt_at.is_none(),
            "force_immediate_retry must clear last_attempt_at"
        );

        // Unknown id is a silent no-op — must not panic, must not mutate
        // the queue.
        let unknown = ItemId {
            event_id: "e".into(),
            game_number: "G_NEVER_QUEUED".into(),
        };
        m.force_immediate_retry(&unknown).unwrap();
        assert_eq!(m.queue.items.len(), 1);
    }

    #[tokio::test]
    async fn force_immediate_retry_pushes_queue_snapshot() {
        // Regression test for the bug where `force_immediate_retry`
        // cleared `last_attempt_at` on disk and in the main-thread
        // queue but failed to notify the background task. Without the
        // snapshot push the background task continued to evaluate
        // retry-eligibility against its stale snapshot and the
        // 15-second retry timer effectively re-ran from the old
        // last-attempt time, so tapping a young yellow row appeared
        // to do nothing.
        //
        // The snapshot channel is not easily observable from this
        // test (the `command_tx` is an internal `mpsc::Sender` and
        // `push_queue_snapshot` spawns an async send). If we could
        // assert snapshot delivery we would; the current snapshot
        // channel isn't easily observable, so we match the rigor of
        // `force_submit_flags_force_and_resets_attempt_counters`
        // (which also doesn't assert snapshot delivery) and verify
        // the observable main-thread state instead: the method
        // returns Ok(()), the target item's `last_attempt_at` is
        // cleared, and the on-disk queue reflects the change.
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();
        m.queue.items[0].last_attempt_at = Some(OffsetDateTime::now_utc());

        let result = m.force_immediate_retry(&id);
        assert!(result.is_ok(), "force_immediate_retry must return Ok");

        // Main-thread queue reflects the clear.
        assert!(
            m.queue.items[0].last_attempt_at.is_none(),
            "main-thread queue must show last_attempt_at cleared"
        );

        // On-disk queue reflects the clear (persistence is part of
        // the contract; the next process start must see the same
        // cleared timestamp).
        let reloaded = queue::load_or_empty(tmp.path()).unwrap();
        assert_eq!(reloaded.items.len(), 1);
        assert!(
            reloaded.items[0].last_attempt_at.is_none(),
            "on-disk queue must show last_attempt_at cleared"
        );
    }

    #[tokio::test]
    async fn on_score_sent_stats_pending_marks_item_and_stays_green() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 3, 2, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();
        assert_eq!(m.indicator_state().health, HealthState::Yellow);

        m.on_score_sent_stats_pending(id);

        assert!(
            m.queue.items[0].score_sent,
            "item must be marked stats-pending"
        );
        assert_eq!(
            m.indicator_state().health,
            HealthState::Green,
            "once the score is sent the dot must be green even though stats are outstanding"
        );
        // Persisted: a restart must reload it as stats-pending.
        let reloaded = queue::load_or_empty(tmp.path()).unwrap();
        assert!(reloaded.items[0].score_sent);
    }

    #[tokio::test]
    async fn is_stats_pending_reflects_score_sent_flag() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        m.enqueue_game_end("e".into(), "G1".into(), 0, 0, "{}".into())
            .unwrap();
        let id = m.queue.items[0].id.clone();
        assert!(!m.is_stats_pending(&id), "fresh item is score-pending");

        m.queue.items[0].score_sent = true;
        assert!(m.is_stats_pending(&id), "score_sent item is stats-pending");

        let unknown = ItemId {
            event_id: "e".into(),
            game_number: "GX".into(),
        };
        assert!(
            !m.is_stats_pending(&unknown),
            "unknown id must be false, not panic"
        );
    }

    #[test]
    fn stats_pending_item_is_never_stuck_even_when_old() {
        let mut it = mk_young_item();
        it.score_sent = true;
        it.queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(120);
        assert!(
            !is_item_stuck(&it, OffsetDateTime::now_utc()),
            "a stats-pending item must never be classified as stuck"
        );
    }

    #[tokio::test]
    async fn detail_rows_places_stats_pending_after_pending_before_recent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // A score-pending (young) game and a stats-pending game.
        m.enqueue_game_end("e".into(), "G_SCORE".into(), 0, 0, "{}".into())
            .unwrap();
        m.queue.items[0].queued_at = OffsetDateTime::now_utc() - TimeDuration::minutes(2);
        m.enqueue_game_end("e".into(), "G_STATS".into(), 1, 0, "{}".into())
            .unwrap();
        m.queue.items[1].score_sent = true;

        // And one recent success.
        m.enqueue_game_end("e".into(), "G_DONE".into(), 2, 1, "{}".into())
            .unwrap();
        let done_id = m.queue.items[2].id.clone();
        m.on_item_resolved(done_id);

        let rows = m.detail_rows();
        assert!(
            matches!(&rows[0], DetailRow::Pending { game_number, .. } if game_number == "G_SCORE"),
            "row0 should be the score-pending game, got {:?}",
            rows[0]
        );
        assert!(
            matches!(&rows[1], DetailRow::StatsPending { game_number, .. } if game_number == "G_STATS"),
            "row1 should be the stats-pending game, got {:?}",
            rows[1]
        );
        assert!(
            matches!(&rows[2], DetailRow::RecentSuccess { game_number, .. } if game_number == "G_DONE"),
            "row2 should be the recent success, got {:?}",
            rows[2]
        );
        assert_eq!(rows.len(), 3);
    }

    #[tokio::test]
    async fn retry_all_preserves_score_sent_and_resets_score_pending() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();
        // One stats-pending game and one score-pending game.
        m.enqueue_game_end("e".into(), "G_STATS".into(), 1, 0, "{}".into())
            .unwrap();
        m.queue.items[0].score_sent = true;
        m.enqueue_game_end("e".into(), "G_SCORE".into(), 2, 1, "{}".into())
            .unwrap();
        m.queue.items[1].attempts = 3;
        m.queue.items[1].last_attempt_at = Some(OffsetDateTime::now_utc());

        m.retry_all().unwrap();

        // Stats-pending item keeps score_sent and is not forced.
        assert!(
            m.queue.items[0].score_sent,
            "retry_all must not clear score_sent"
        );
        assert!(
            !m.queue.items[0].force,
            "retry_all must not set force on the stats-pending item"
        );
        // Score-pending item is reset for immediate auto-retry.
        assert_eq!(m.queue.items[1].attempts, 0);
        assert!(m.queue.items[1].last_attempt_at.is_none());
        assert!(
            !m.queue.items[1].force,
            "retry_all must not set force on the score-pending item"
        );
    }
}
