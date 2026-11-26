//! Background health-check task and retry loop.
//!
//! The decision logic (when to fire a health check, when to retry an item,
//! what cadence to use) is encapsulated in `HealthDecisionState` and is
//! unit-testable without any async runtime or I/O. The async side
//! (`spawn`, `run_task`, `PortalTaskIo`) drives that decision logic from
//! a Tokio task; see `BackgroundTaskHandle` for the UI-side integration
//! surface.

use std::time::Duration;

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::sync::mpsc;

use super::HealthState;
use super::PortalEvent;
use super::queue::{QueueFile, QueuedItem, QueuedSwap};
use super::{SWAP_HOLD_LIMIT, is_item_stuck, is_swap_stuck};
use uwh_common::uwhportal::ColourSwapOutcome;

pub const GREEN_CADENCE: Duration = Duration::from_secs(5 * 60);
pub const DEGRADED_CADENCE: Duration = Duration::from_secs(15);

/// How often `run_task` wakes to re-evaluate cadence and drain commands.
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// How long after a failed attempt the background task waits before
/// retrying the same queued item. Exposed so the detail-page view can
/// compute a live "retry in 0:NN" countdown from `last_attempt_at`.
pub const ITEM_RETRY_INTERVAL: TimeDuration = TimeDuration::seconds(15);

pub fn is_item_retry_eligible(item: &QueuedItem, now: OffsetDateTime) -> bool {
    if is_item_stuck(item, now) {
        return false; // Stuck items wait for operator action.
    }
    match item.last_attempt_at {
        None => true,
        Some(last) => (now - last) >= ITEM_RETRY_INTERVAL,
    }
}

/// The same rule as `is_item_retry_eligible`, for a queued colour swap: a
/// stuck swap waits for RETRY ALL, otherwise one attempt per
/// `ITEM_RETRY_INTERVAL`.
pub fn is_swap_retry_eligible(swap: &QueuedSwap, now: OffsetDateTime) -> bool {
    if is_swap_stuck(swap, now) {
        return false; // Stuck swaps wait for operator action.
    }
    match swap.last_attempt_at {
        None => true,
        Some(last) => (now - last) >= ITEM_RETRY_INTERVAL,
    }
}

/// True while `item`'s game still has a colour swap queued that is not stuck.
/// Its score waits: the site must learn which team played dark before it
/// records the result. Matches on event AND game, so a swap never holds
/// another event's game of the same number.
///
/// The wait is measured on the SCORE's own clock: once the score has been
/// queued for `SWAP_HOLD_LIMIT` (20 minutes, below the 30-minute stuck
/// threshold) it goes even if the colours were not recorded, so a final score
/// is never held back for good by a colours call that keeps failing, and it is
/// released while it still retries by itself. Queuing the score re-armed its
/// swap (`enqueue_game_end`), so the swap retries automatically throughout
/// the wait. A stuck swap never holds a score; it stays queued, red, and
/// retryable by RETRY ALL.
pub(crate) fn score_waits_for_swap(
    item: &QueuedItem,
    swaps: &[QueuedSwap],
    now: OffsetDateTime,
) -> bool {
    if (now - item.queued_at) >= SWAP_HOLD_LIMIT {
        return false;
    }
    swaps
        .iter()
        .any(|swap| swap.id == item.id && !is_swap_stuck(swap, now))
}

/// The `status` of a refusal this refbox makes itself, without sending: a
/// queued swap whose ids can never be parsed. No HTTP answer carries 0, so it
/// cannot be mistaken for the site's own refusal.
pub(crate) const LOCAL_REFUSAL_STATUS: u16 = 0;

/// What happens to a queued swap after one send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwapStep {
    /// The site recorded the colours. Remove the swap; the score may go.
    Delivered,
    /// The site answered and will never take this swap (refused, or has no
    /// colours call). Remove it; the swap stays on this refbox only.
    Dropped,
    /// No usable answer. Keep the swap queued and try again later.
    Retry,
}

pub(crate) fn swap_step(result: &Result<ColourSwapOutcome, PortalCallError>) -> SwapStep {
    match result {
        Ok(ColourSwapOutcome::Applied) => SwapStep::Delivered,
        Ok(ColourSwapOutcome::Refused { .. }) | Ok(ColourSwapOutcome::NotAccepted { .. }) => {
            SwapStep::Dropped
        }
        Ok(ColourSwapOutcome::Retry { .. }) | Err(_) => SwapStep::Retry,
    }
}

/// The one log line for each colour-swap send. The operator is never shown a
/// refusal, so this is the only record of why a swap stayed on this refbox.
fn log_swap_outcome(swap: &QueuedSwap, result: &Result<ColourSwapOutcome, PortalCallError>) {
    let game = &swap.id.game_number;
    match result {
        Ok(ColourSwapOutcome::Applied) => {
            log::info!("Game {game}: colour swap recorded on the site");
        }
        Ok(ColourSwapOutcome::Refused { status, reason }) if *status == LOCAL_REFUSAL_STATUS => {
            log::warn!(
                "Game {game}: colour swap could not be sent ({reason}); kept on this refbox only"
            );
        }
        Ok(ColourSwapOutcome::Refused { status, reason }) => {
            log::warn!(
                "Game {game}: site refused the colour swap ({status} {reason}); kept on this refbox only"
            );
        }
        Ok(ColourSwapOutcome::NotAccepted { status }) => {
            log::info!(
                "Game {game}: site does not accept colour swaps ({status}); kept on this refbox only"
            );
        }
        Ok(ColourSwapOutcome::Retry { status }) => {
            log::warn!("colour swap upload for game {game}: site answered {status}, will retry");
        }
        Err(e) => {
            log::warn!("colour swap upload for game {game}: {e}");
        }
    }
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
    /// Refresh the task's view of the queue. Sent by `PortalManager`
    /// after every queue mutation so the task knows which items to
    /// attempt on its next tick. `epoch` is the discard counter (`DiscardEpoch`) the snapshot
    /// was taken under; the task ignores a snapshot older than the one it holds, and passes the
    /// value to every post it makes from it.
    QueueUpdated { queue: QueueFile, epoch: u64 },
    /// One-shot request to (re)post the stats for a single stats-pending
    /// item. Triggered by the operator tapping the item's row or RETRY
    /// ALL. The task attempts stats exactly once and never schedules a
    /// follow-up — stats-pending items are not on the auto-retry cadence.
    /// `epoch` is the discard counter the item was handed over under, passed to the post.
    RetryStats { item: QueuedItem, epoch: u64 },
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
    /// `epoch` is the discard counter the item was handed to the task under. A production
    /// implementation refuses the post, without sending, if the queue has been discarded since.
    async fn post_scores(&self, item: &QueuedItem, epoch: u64) -> Result<(), PortalCallError>;
    async fn post_stats(&self, item: &QueuedItem, epoch: u64) -> Result<(), PortalCallError>;
    /// Send one queued colour swap. `epoch` works exactly as for `post_scores`. `Err` means the
    /// site gave no answer at all (or the post was refused before sending).
    async fn post_colours(
        &self,
        swap: &QueuedSwap,
        epoch: u64,
    ) -> Result<ColourSwapOutcome, PortalCallError>;
}

#[derive(Debug)]
pub enum PortalCallError {
    /// The portal responded, but with a non-success status. The current
    /// uwh-common API keeps no status code, so a conflict (409), token
    /// rejection (401), or server error (5xx) all collapse to this one
    /// variant and can be told apart only by the response body it carries
    /// (as `portal_refuses_stats_for_event` and
    /// `portal_already_has_these_stats` do). See ADR 011 amendment
    /// (2026-04-21) for the rationale.
    Failed(String),
    /// The portal could not be reached at all: the HTTP exchange never
    /// completed (DNS, connect, timeout, TLS, or a dropped response). The
    /// login may be perfectly valid — this is a connectivity problem, not a
    /// token problem — so callers must not push the operator to re-login.
    /// Produced by `classify_error` downcasting the underlying `reqwest`
    /// transport error.
    Unreachable(String),
    /// The request was rejected before any socket was opened, so nothing
    /// ever left the machine. The case this exists for is `https_only`
    /// refusing a plain-http address, but an address that cannot be used at
    /// all (no host, or a scheme that is not http or https) lands here too.
    ///
    /// Deliberately classified by that shared fact — never sent — rather
    /// than by cause: only the launch-time check knows whether https is
    /// required, so naming a cause here would let the log tell an operator
    /// who already passed `--allow-http` to go and pass `--allow-http`.
    ///
    /// This is neither a connectivity failure nor a token problem, and
    /// reporting it as `Unreachable` sends the operator hunting a network
    /// fault that does not exist. Handled exactly like `Unreachable`
    /// downstream (indicator red, no re-login prompt); only the wording
    /// differs. Produced by `classify_error`.
    NotSent(String),
}

impl std::fmt::Display for PortalCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(msg) => write!(f, "portal call failed: {msg}"),
            Self::Unreachable(msg) => write!(f, "portal unreachable: {msg}"),
            Self::NotSent(msg) => write!(f, "portal request was never sent: {msg}"),
        }
    }
}

/// The reason the portal gives, word for word, when it refuses stats because
/// the event does not require unique cap numbers (portal api tag 1.0.0.1192,
/// `AdminController.EventStats.cs`). It checks that setting on every upload,
/// after validating the items, so even a game with nothing to report gets it.
const STATS_REFUSED_FOR_EVENT: &str = "This event does not require unique cap numbers.";

/// True when the portal has said it will never take stats for this event, so
/// there is nothing left to send and the item is done.
///
/// Only that exact reason counts. Every other failure, including a validation
/// error, keeps the item stats-pending as before (a duplicate has its own,
/// narrower rule: `portal_already_has_these_stats`). If the
/// portal ever rewords the reason, this stops matching and the row goes back
/// to "Stats not sent", which the operator can still clear by hand.
fn portal_refuses_stats_for_event(e: &PortalCallError) -> bool {
    portal_reason_is(e, STATS_REFUSED_FOR_EVENT)
}

/// The reason the portal gives when a stats upload contains an event it
/// already stores.
const STATS_ALREADY_STORED: &str =
    "Event is a duplicate, based on Type, PlayerCapNumber, Side and OccurredOn";

/// True when the portal answered a stats upload with its "duplicate" refusal.
///
/// That answer proves only that SOME event in the upload is already stored,
/// and the portal saved nothing from this attempt: a game ended twice queues
/// a second row that repeats the first row's events, and the portal refuses
/// it whole. It counts as "already saved" only for a row whose earlier stats
/// send went unanswered (`stats_may_have_landed`): the portal saves an upload
/// all-or-nothing and the refbox resends the identical stored body, so then
/// it means that unanswered send was saved in full. Only the portal's exact
/// reason counts: any other wording, a body that merely quotes it, or an
/// error where the portal never answered leaves the row as before.
fn portal_already_has_these_stats(e: &PortalCallError) -> bool {
    portal_reason_is(e, STATS_ALREADY_STORED)
}

/// True when the portal answered with exactly `expected` as the `reason` of
/// its JSON body. Any other body, or an error where the portal never
/// answered, is not that answer.
fn portal_reason_is(e: &PortalCallError, expected: &str) -> bool {
    let PortalCallError::Failed(body) = e else {
        return false;
    };
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("reason")?.as_str().map(|r| r == expected))
        .unwrap_or(false)
}

async fn run_task(
    io: impl PortalTaskIo,
    mut command_rx: mpsc::Receiver<PortalCommand>,
    event_tx: mpsc::Sender<PortalEvent>,
) {
    use tokio::time::Instant as TokioInstant;
    use tokio::time::sleep;

    let mut last_success: Option<TokioInstant> = None;
    // Stamped on every `verify_token` attempt regardless of outcome.
    // Used as the cadence reference once the indicator is degraded
    // (Yellow/Red), where "wait 15 seconds between probes" is the
    // intended behaviour — without this, a long-running outage would
    // re-fire `verify_token` on every 2-second poll because
    // `last_success` never advances.
    let mut last_check_at: Option<TokioInstant> = None;
    let mut current_health = HealthState::Green;
    // Whether the "never sent" warning has already been logged for the run of
    // checks we are in. Cleared by any check that produces a different
    // outcome, so the warning returns if the condition returns.
    let mut not_sent_logged = false;
    let mut queue_snapshot = QueueFile::empty();
    // The discard counter `queue_snapshot` was taken under. Each `QueueUpdated` is sent from its
    // own spawned task, so two can arrive out of order; refusing an older one keeps a snapshot
    // from before a discard from bringing the discarded results back.
    let mut snapshot_epoch: u64 = 0;

    loop {
        tokio::select! {
            _ = sleep(POLL_INTERVAL) => {
                // Tick: try each eligible colour swap, then each eligible
                // queued item, then health-check if due. A score whose game
                // still has a swap in this snapshot is skipped: it goes on a
                // later tick, once the main thread has dropped the resolved
                // swap and pushed a fresh snapshot.
                let now = OffsetDateTime::now_utc();
                for idx in 0..queue_snapshot.swaps.len() {
                    let swap = &queue_snapshot.swaps[idx];
                    if !is_swap_retry_eligible(swap, now) {
                        continue;
                    }
                    let result = io.post_colours(swap, snapshot_epoch).await;
                    log_swap_outcome(swap, &result);
                    if matches!(result, Ok(ColourSwapOutcome::Applied)) {
                        last_success = Some(TokioInstant::now());
                    }
                    // Stamped on our own copy whatever the outcome, as for
                    // score items: a resolved swap stays in this snapshot (and
                    // keeps holding its score) until the main thread drops it
                    // and pushes a fresh one, and must not be re-sent every
                    // poll meanwhile.
                    let swap = &mut queue_snapshot.swaps[idx];
                    swap.attempts = swap.attempts.saturating_add(1);
                    swap.last_attempt_at = Some(now);
                    let event = match swap_step(&result) {
                        SwapStep::Delivered => PortalEvent::SwapResolved {
                            id: swap.id.clone(),
                            dark_team_id: swap.dark_team_id.clone(),
                            light_team_id: swap.light_team_id.clone(),
                            delivered: true,
                        },
                        SwapStep::Dropped => PortalEvent::SwapResolved {
                            id: swap.id.clone(),
                            dark_team_id: swap.dark_team_id.clone(),
                            light_team_id: swap.light_team_id.clone(),
                            delivered: false,
                        },
                        SwapStep::Retry => PortalEvent::SwapAttempted {
                            id: swap.id.clone(),
                            dark_team_id: swap.dark_team_id.clone(),
                            light_team_id: swap.light_team_id.clone(),
                            attempts: swap.attempts,
                            at: now,
                        },
                    };
                    let _ = event_tx.send(event).await;
                }
                for idx in 0..queue_snapshot.items.len() {
                    let item = &queue_snapshot.items[idx];
                    if item.score_sent
                        || !is_item_retry_eligible(item, now)
                        || score_waits_for_swap(item, &queue_snapshot.swaps, now)
                    {
                        continue;
                    }
                    if attempt_item(&io, item, snapshot_epoch, &event_tx).await {
                        last_success = Some(TokioInstant::now());
                    }
                    // Record the attempt on our own snapshot copy so this task
                    // self-throttles to ITEM_RETRY_INTERVAL: `is_item_retry_eligible`
                    // keys off `last_attempt_at`, so without stamping it here the item
                    // would look eligible on every POLL_INTERVAL tick and we'd re-send
                    // it every ~2s instead of every ~15s. Also notify the main thread
                    // so it mirrors the attempt onto the authoritative queue (the honest
                    // "(attempt N)" counter and the detail-page retry countdown). See
                    // finding H4.
                    let item = &mut queue_snapshot.items[idx];
                    item.attempts = item.attempts.saturating_add(1);
                    item.last_attempt_at = Some(now);
                    let _ = event_tx
                        .send(PortalEvent::ItemAttempted {
                            id: item.id.clone(),
                            attempts: item.attempts,
                            at: now,
                        })
                        .await;
                }

                let elapsed = match current_health {
                    HealthState::Green => last_success
                        .map(|t| TokioInstant::now().saturating_duration_since(t)),
                    HealthState::Yellow | HealthState::Red => last_check_at
                        .map(|t| TokioInstant::now().saturating_duration_since(t)),
                };
                let state = HealthDecisionState { current_health };
                if state.is_health_check_due(elapsed) {
                    let _ = event_tx.send(PortalEvent::HealthChanged).await;
                    last_check_at = Some(TokioInstant::now());
                    match io.verify_token().await {
                        Ok(()) => {
                            last_success = Some(TokioInstant::now());
                            current_health = HealthState::Green;
                            not_sent_logged = false;
                            let _ = event_tx.send(PortalEvent::TokenStatus(true)).await;
                        }
                        Err(PortalCallError::Unreachable(e)) => {
                            // Couldn't reach the portal (wifi blip / outage).
                            // Degrade the indicator, but do NOT report a token
                            // problem: the login may be fine, so the UI must
                            // not push the operator to re-login or grey out the
                            // schedule REFRESH button.
                            //
                            // Log it: the indicator turning red is the only
                            // other signal, and it cannot say WHY or when it
                            // started. Every failed check is logged rather than
                            // just the first, so the log shows how long the
                            // outage ran — at the 15s degraded cadence that is
                            // roughly 30KB an hour against a rolling log.
                            log::warn!("portal health check could not reach the portal: {e}");
                            current_health = HealthState::Red;
                            not_sent_logged = false;
                            let _ = event_tx.send(PortalEvent::TokenUnreachable).await;
                        }
                        Err(PortalCallError::NotSent(e)) => {
                            // Nothing reached the network, so degrade exactly
                            // as an outage does — red, and no token verdict,
                            // since the token was never offered to anyone —
                            // but say what happened instead of blaming the
                            // network for a configuration choice.
                            //
                            // Logged once, unlike the unreachable arm above.
                            // That arm repeats deliberately, because how long
                            // an outage ran is worth knowing; here the address
                            // is fixed at launch and cannot start working
                            // mid-session, so repeating would add ~240
                            // identical lines an hour and say nothing new. The
                            // flag clears on any other outcome, so a later
                            // recurrence is logged again.
                            if !not_sent_logged {
                                log::warn!("portal health check was never sent: {e}");
                                not_sent_logged = true;
                            }
                            current_health = HealthState::Red;
                            let _ = event_tx.send(PortalEvent::TokenUnreachable).await;
                        }
                        Err(PortalCallError::Failed(_)) => {
                            // The portal responded but rejected the request (or
                            // returned a server error). Treat as a token
                            // problem so the operator is prompted to re-login.
                            current_health = HealthState::Red;
                            not_sent_logged = false;
                            let _ = event_tx.send(PortalEvent::TokenStatus(false)).await;
                        }
                    }
                }
            }
            cmd = command_rx.recv() => {
                match cmd {
                    None => break,
                    Some(PortalCommand::QueueUpdated { queue, epoch }) => {
                        if epoch >= snapshot_epoch {
                            queue_snapshot = queue;
                            snapshot_epoch = epoch;
                        }
                    }
                    Some(PortalCommand::RetryStats { item, epoch }) => {
                        match io.post_stats(&item, epoch).await {
                            Ok(()) => {
                                let _ = event_tx
                                    .send(PortalEvent::ItemResolved(item.id.clone()))
                                    .await;
                            }
                            Err(e) if portal_refuses_stats_for_event(&e) => {
                                // Same rule as `attempt_item`: the event keeps
                                // no stats, so the item is done. Reached by any
                                // stats-pending row, whatever stuck it first.
                                log::info!(
                                    "stats for game {} not kept by the portal for this event: {e}",
                                    item.id.game_number
                                );
                                let _ = event_tx
                                    .send(PortalEvent::ItemResolved(item.id.clone()))
                                    .await;
                            }
                            Err(e)
                                if item.stats_may_have_landed
                                    && portal_already_has_these_stats(&e) =>
                            {
                                // The earlier unanswered send was saved in
                                // full; nothing is left to send.
                                log::info!(
                                    "stats for game {} already on the portal: {e}",
                                    item.id.game_number
                                );
                                let _ = event_tx
                                    .send(PortalEvent::ItemResolved(item.id.clone()))
                                    .await;
                            }
                            Err(e) => {
                                // Stays stats-pending; no escalation, no loop.
                                // Logged for the same reason as the first
                                // attempt: a manual retry that fails again
                                // otherwise reports nothing at all.
                                log::warn!(
                                    "stats retry for game {}: {e}",
                                    item.id.game_number
                                );
                                if matches!(e, PortalCallError::Unreachable(_)) {
                                    // No answer came back: the portal may have
                                    // saved it. Marked before ItemUpdated.
                                    let _ = event_tx
                                        .send(PortalEvent::StatsMayHaveLanded(item.id.clone()))
                                        .await;
                                }
                                let _ = event_tx.send(PortalEvent::ItemUpdated).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Attempt to submit a single queued item (scores + stats). The portal
/// API collapses its non-success answers (409 conflict, 401 token
/// expired, 5xx) into a single `Failed` error, told apart only by the body
/// it carries. Five outcomes:
///
/// - **Score fails** → emits `ItemUpdated`, returns `false`. The item
///   remains fully on the queue; `last_success` is not advanced so the
///   cadence-driven `verify_token` health check is not suppressed.
/// - **Score succeeds, stats fail** → emits `ScoreSentStatsPending`,
///   returns `true`. The portal is reachable (score posted), so we
///   advance `last_success` to suppress an unnecessary `verify_token`.
///   The main thread flips the item to `score_sent = true` so it exits
///   the auto-retry loop and the yellow/red indicator.
/// - **Score succeeds, portal refuses stats for the whole event** → emits
///   `ItemResolved`, returns `true`. See `portal_refuses_stats_for_event`.
/// - **Score succeeds, portal already holds a marked row's stats** → emits
///   `ItemResolved`, returns `true`. See `portal_already_has_these_stats`.
/// - **Both succeed** → emits `ItemResolved`, returns `true`.
///
/// The portal's "duplicate" answer counts as already saved only for a row
/// whose earlier stats send went unanswered (`stats_may_have_landed`), here
/// or on `RetryStats`. A row reaches this path marked only when the app
/// closed before the row turned stats-pending. When the stats send itself
/// gets no answer (`Unreachable`), `StatsMayHaveLanded` is emitted before
/// `ScoreSentStatsPending`.
async fn attempt_item(
    io: &impl PortalTaskIo,
    item: &QueuedItem,
    epoch: u64,
    event_tx: &mpsc::Sender<PortalEvent>,
) -> bool {
    let score_result = io.post_scores(item, epoch).await;
    if let Err(e) = score_result {
        // The portal's own words are the only thing separating a rejected
        // upload from an unreachable one, and the error already carries them
        // (`classify_error` puts the response body in `Failed`). Discarding it
        // here left the operator with a red item and nothing to diagnose from.
        // Wording note: `PortalCallError` already renders its own
        // "portal call failed:" / "portal unreachable:" prefix, so this message
        // supplies only what was being sent and for which game.
        log::warn!("score upload for game {}: {e}", item.id.game_number);
        let _ = event_tx.send(PortalEvent::ItemUpdated).await;
        return false;
    }
    match io.post_stats(item, epoch).await {
        Ok(()) => {
            let _ = event_tx
                .send(PortalEvent::ItemResolved(item.id.clone()))
                .await;
            true
        }
        Err(e) if portal_refuses_stats_for_event(&e) => {
            // The event does not keep stats at all, so there is nothing
            // left to send: resolve rather than leave "Stats not sent".
            log::info!(
                "stats for game {} not kept by the portal for this event: {e}",
                item.id.game_number
            );
            let _ = event_tx
                .send(PortalEvent::ItemResolved(item.id.clone()))
                .await;
            true
        }
        Err(e) if item.stats_may_have_landed && portal_already_has_these_stats(&e) => {
            // An earlier unanswered send of this row was saved in full.
            log::info!(
                "stats for game {} already on the portal: {e}",
                item.id.game_number
            );
            let _ = event_tx
                .send(PortalEvent::ItemResolved(item.id.clone()))
                .await;
            true
        }
        Err(e) => {
            // Score is up but stats failed. Mark the item stats-pending; the
            // portal is reachable, so return `true` to suppress the cadence
            // `verify_token`.
            //
            // Logged because the reason is not otherwise recoverable: the row
            // only ever says "Stats not sent", and the several causes the
            // portal distinguishes are indistinguishable from the UI.
            log::warn!(
                "stats upload for game {} (score already accepted): {e}",
                item.id.game_number
            );
            if matches!(e, PortalCallError::Unreachable(_)) {
                // No answer came back: the portal may have saved it. Marked
                // first so the row is stats-pending with the flag already set.
                let _ = event_tx
                    .send(PortalEvent::StatsMayHaveLanded(item.id.clone()))
                    .await;
            }
            let _ = event_tx
                .send(PortalEvent::ScoreSentStatsPending(item.id.clone()))
                .await;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ItemId;
    use super::*;

    /// The reason the portal gave reaches the log only through this error's own
    /// text, which the upload-failure `warn!`s print verbatim. Nothing else
    /// asserts that, so if `Display` ever stopped including `msg` the logs would
    /// go quiet again — the exact defect those `warn!`s were added to fix — with
    /// every other test still green.
    #[test]
    fn error_text_carries_the_reason_the_portal_gave() {
        let reason = r#"{"reason":"This event does not require unique cap numbers."}"#;
        assert!(
            PortalCallError::Failed(reason.to_string())
                .to_string()
                .contains(reason),
            "a rejected upload must keep the portal's wording"
        );
        assert!(
            PortalCallError::Unreachable("dns failure".to_string())
                .to_string()
                .contains("dns failure"),
            "an unreachable portal must keep the transport detail"
        );
        assert!(
            PortalCallError::NotSent("http://127.0.0.1:9099 is not an https address".to_string())
                .to_string()
                .contains("http://127.0.0.1:9099"),
            "a blocked request must keep the address that was refused"
        );
    }

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
            score_sent: false,
            stats_may_have_landed: false,
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

    /// The epochs a `FakeIo` was handed, one per call, in order.
    type EpochLog = Arc<Mutex<Vec<u64>>>;

    struct FakeIo {
        verify_results: Mutex<Vec<Result<(), PortalCallError>>>,
        verify_count: Arc<std::sync::atomic::AtomicU32>,
        scores_results: Mutex<Vec<Result<(), PortalCallError>>>,
        scores_count: Arc<std::sync::atomic::AtomicU32>,
        stats_results: Mutex<Vec<Result<(), PortalCallError>>>,
        stats_count: Arc<std::sync::atomic::AtomicU32>,
        /// The epoch passed with each `post_scores` call, in order.
        scores_epochs: EpochLog,
        /// The epoch passed with each `post_stats` call, in order.
        stats_epochs: EpochLog,
    }

    #[async_trait::async_trait]
    impl PortalTaskIo for FakeIo {
        async fn verify_token(&self) -> Result<(), PortalCallError> {
            self.verify_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.verify_results.lock().unwrap();
            if v.is_empty() { Ok(()) } else { v.remove(0) }
        }
        async fn post_scores(&self, _: &QueuedItem, epoch: u64) -> Result<(), PortalCallError> {
            self.scores_epochs.lock().unwrap().push(epoch);
            self.scores_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.scores_results.lock().unwrap();
            if v.is_empty() { Ok(()) } else { v.remove(0) }
        }
        async fn post_stats(&self, _: &QueuedItem, epoch: u64) -> Result<(), PortalCallError> {
            self.stats_epochs.lock().unwrap().push(epoch);
            self.stats_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.stats_results.lock().unwrap();
            if v.is_empty() { Ok(()) } else { v.remove(0) }
        }
        async fn post_colours(
            &self,
            _: &QueuedSwap,
            _: u64,
        ) -> Result<ColourSwapOutcome, PortalCallError> {
            Ok(ColourSwapOutcome::Applied)
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
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let _handle = spawn(io);

        // Two rounds of (advance + yield) are needed: the first wakes the
        // select's timer arm and lets `verify_token`'s future be polled;
        // the second gives that future a chance to resolve before we read
        // the counter. With `last_success = None` at startup, the first
        // cadence tick is immediately due and the health check fires.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;

        assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 1);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn unreachable_verify_emits_token_unreachable_not_token_status() {
        // A verify_token that cannot reach the portal must surface as
        // TokenUnreachable (a connectivity problem), never TokenStatus(false)
        // — which the UI reads as "login expired" and uses to grey out the
        // schedule REFRESH button.
        let io = FakeIo {
            verify_results: Mutex::new(vec![Err(PortalCallError::Unreachable("blip".into()))]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::TokenUnreachable)),
            "unreachable verify must emit TokenUnreachable, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::TokenStatus(_))),
            "unreachable verify must NOT emit TokenStatus, got {events:?}"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn unsent_verify_emits_token_unreachable_not_token_status() {
        // A verify_token the refbox refused to send is not evidence the login
        // expired — the token was never offered to anyone. It must degrade the
        // indicator exactly like an outage does (TokenUnreachable) and must NOT
        // emit TokenStatus(false), which the UI reads as "login expired" and
        // uses to push a pointless re-login.
        let io = FakeIo {
            verify_results: Mutex::new(vec![Err(PortalCallError::NotSent(
                "http://127.0.0.1:9099 is not an https address".into(),
            ))]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::TokenUnreachable)),
            "unsent verify must emit TokenUnreachable, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::TokenStatus(_))),
            "unsent verify must NOT emit TokenStatus, got {events:?}"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn rejected_verify_emits_token_status_false() {
        // A verify_token that reached the portal but was rejected must
        // surface as TokenStatus(false) so the operator is prompted to
        // re-login — and must NOT be confused with an outage.
        let io = FakeIo {
            verify_results: Mutex::new(vec![Err(PortalCallError::Failed("401".into()))]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::TokenStatus(false))),
            "rejected verify must emit TokenStatus(false), got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::TokenUnreachable)),
            "rejected verify must NOT emit TokenUnreachable, got {events:?}"
        );
        drop(handle);
    }

    /// Build a `QueueFile` with a single freshly-queued item (no prior
    /// attempts), which makes that item immediately retry-eligible by
    /// `is_item_retry_eligible`.
    fn queue_with_one_eligible_item() -> QueueFile {
        QueueFile {
            version: QueueFile::CURRENT_VERSION,
            items: vec![mk_queue_item(0)],
            swaps: Vec::new(),
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
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
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

        drop(handle);
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
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
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
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated)),
            "expected ItemUpdated event after failed scores, got {events:?}"
        );

        drop(handle.command_tx);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn score_ok_stats_fail_emits_score_sent_stats_pending() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Err(PortalCallError::Failed("server error".into()))]),
            stats_count: stats_count.clone(),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
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

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                PortalEvent::ScoreSentStatsPending(id) if id == &expected_id
            )),
            "expected ScoreSentStatsPending for {expected_id:?}, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemResolved(_))),
            "score-ok/stats-fail must NOT resolve the item"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn stats_pending_item_is_not_auto_attempted_by_loop() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Ok(())]),
            stats_count: stats_count.clone(),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let handle = spawn(io);

        // A single item already in the stats-pending state.
        let mut item = mk_queue_item(0);
        item.score_sent = true;
        let queue = QueueFile {
            version: QueueFile::CURRENT_VERSION,
            items: vec![item],
            swaps: Vec::new(),
        };
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;

        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "the auto-retry loop must not re-post scores for a stats-pending item"
        );
        assert_eq!(
            stats_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "the auto-retry loop must not auto-attempt stats for a stats-pending item"
        );
        drop(handle);
    }

    /// The portal's body when an event does not require unique cap numbers.
    const REFUSED_FOR_EVENT_BODY: &str =
        r#"{"reason":"This event does not require unique cap numbers."}"#;

    #[test]
    fn only_the_portals_exact_refusal_means_stats_are_never_wanted() {
        assert!(portal_refuses_stats_for_event(&PortalCallError::Failed(
            REFUSED_FOR_EVENT_BODY.into()
        )));
        // Anything else may still be fixed or worth a retry, so it must stay
        // stats-pending: a duplicate, a validation error, another reason
        // that happens to quote the sentence, or a body that is not JSON.
        for body in [
            r#"{"reason":"Event is a duplicate, based on Type, PlayerCapNumber, Side and OccurredOn"}"#,
            r#"{"errors":{"[0].Type":["'Type' must not be empty."]}}"#,
            r#"{"reason":"Unrelated: This event does not require unique cap numbers."}"#,
            "This event does not require unique cap numbers.",
            "",
        ] {
            assert!(
                !portal_refuses_stats_for_event(&PortalCallError::Failed(body.into())),
                "{body:?} must not count as the portal's refusal"
            );
        }
        // The portal never answered, so it has refused nothing.
        assert!(!portal_refuses_stats_for_event(
            &PortalCallError::Unreachable(REFUSED_FOR_EVENT_BODY.into())
        ));
        assert!(!portal_refuses_stats_for_event(&PortalCallError::NotSent(
            REFUSED_FOR_EVENT_BODY.into()
        )));
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn stats_refused_for_the_event_resolves_the_item_on_first_attempt() {
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![Err(PortalCallError::Failed(
                REFUSED_FOR_EVENT_BODY.into(),
            ))]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev, PortalEvent::ItemResolved(id) if id == &expected_id
            )),
            "a refusal for the whole event must resolve the item, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ScoreSentStatsPending(_))),
            "a refusal for the whole event must not leave Stats not sent, got {events:?}"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn stats_refused_for_the_event_resolves_the_item_on_retry() {
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![Err(PortalCallError::Failed(
                REFUSED_FOR_EVENT_BODY.into(),
            ))]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        // A row left stuck by a build without this rule, then retried.
        let mut item = mk_queue_item(0);
        item.score_sent = true;
        let expected_id = item.id.clone();
        handle
            .command_tx
            .send(PortalCommand::RetryStats { item, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev, PortalEvent::ItemResolved(id) if id == &expected_id
            )),
            "a refusal for the whole event must resolve a retried item, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated)),
            "a refusal for the whole event must not keep the row, got {events:?}"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn retry_stats_command_posts_stats_once_and_resolves_on_success() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Ok(())]),
            stats_count: stats_count.clone(),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let mut item = mk_queue_item(0);
        item.score_sent = true;
        let expected_id = item.id.clone();
        handle
            .command_tx
            .send(PortalCommand::RetryStats { item, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(
            stats_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "RetryStats must post stats exactly once"
        );
        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "RetryStats must never re-post the score"
        );
        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev, PortalEvent::ItemResolved(id) if id == &expected_id
            )),
            "successful RetryStats must resolve the item, got {events:?}"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn retry_stats_command_emits_item_updated_on_failure() {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![Err(PortalCallError::Failed("forced".into()))]),
            stats_count: stats_count.clone(),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let mut item = mk_queue_item(0);
        item.score_sent = true;
        handle
            .command_tx
            .send(PortalCommand::RetryStats { item, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(
            stats_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "RetryStats must post stats exactly once even on failure"
        );
        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "RetryStats must never re-post the score"
        );
        let events = drain_events(&mut handle.event_rx);
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated)),
            "failed RetryStats must emit ItemUpdated, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemResolved(_))),
            "failed RetryStats must NOT emit ItemResolved, got {events:?}"
        );
        drop(handle);
    }

    const DUP_BODY: &str = r#"{"reason":"Event is a duplicate, based on Type, PlayerCapNumber, Side and OccurredOn","event":{"type":"Goal"}}"#;

    #[test]
    fn only_the_portals_duplicate_answer_means_stats_are_already_stored() {
        assert!(portal_already_has_these_stats(&PortalCallError::Failed(
            DUP_BODY.to_string()
        )));
        for body in [
            r#"{"reason":"This event does not require unique cap numbers."}"#,
            r#"{"errors":{"[0].Type":["'Type' must not be empty."]}}"#,
            r#"{"reason":"Unrelated: Event is a duplicate, based on Type, PlayerCapNumber, Side and OccurredOn"}"#,
            "Event is a duplicate, based on Type, PlayerCapNumber, Side and OccurredOn",
            "",
        ] {
            assert!(
                !portal_already_has_these_stats(&PortalCallError::Failed(body.to_string())),
                "must not match: {body}"
            );
        }
        // The portal never answered: the text proves nothing.
        assert!(!portal_already_has_these_stats(
            &PortalCallError::Unreachable(DUP_BODY.to_string())
        ));
        assert!(!portal_already_has_these_stats(&PortalCallError::NotSent(
            DUP_BODY.to_string()
        )));
    }

    /// Run one first attempt (`attempt_item` via the retry tick) of a fresh
    /// item whose score posts and whose stats get `stats_result`. Returns the
    /// item's id and every event emitted.
    async fn first_attempt_with_stats(
        stats_result: Result<(), PortalCallError>,
    ) -> (ItemId, Vec<PortalEvent>) {
        first_attempt_of(false, stats_result).await
    }

    /// As `first_attempt_with_stats`, for an item with the given
    /// `stats_may_have_landed` flag.
    async fn first_attempt_of(
        may_have_landed: bool,
        stats_result: Result<(), PortalCallError>,
    ) -> (ItemId, Vec<PortalEvent>) {
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Ok(())]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![stats_result]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let mut queue = queue_with_one_eligible_item();
        queue.items[0].stats_may_have_landed = may_have_landed;
        let id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        drop(handle);
        (id, events)
    }

    /// Send one `RetryStats` for a stats-pending item (`score_sent = true`)
    /// with the given `stats_may_have_landed` flag; its stats get
    /// `stats_result`. Returns the item's id and every event emitted, after
    /// asserting the score was never re-posted.
    async fn retry_stats_with(
        may_have_landed: bool,
        stats_result: Result<(), PortalCallError>,
    ) -> (ItemId, Vec<PortalEvent>) {
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let stats_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![Ok(())]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![stats_result]),
            stats_count: stats_count.clone(),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);

        let mut item = mk_queue_item(0);
        item.score_sent = true;
        item.stats_may_have_landed = may_have_landed;
        let id = item.id.clone();
        handle
            .command_tx
            .send(PortalCommand::RetryStats { item, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(stats_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "RetryStats must never re-post the score"
        );
        let events = drain_events(&mut handle.event_rx);
        drop(handle);
        (id, events)
    }

    fn resolved(events: &[PortalEvent], id: &ItemId) -> bool {
        events
            .iter()
            .any(|ev| matches!(ev, PortalEvent::ItemResolved(i) if i == id))
    }

    fn marked_may_have_landed(events: &[PortalEvent]) -> bool {
        events
            .iter()
            .any(|ev| matches!(ev, PortalEvent::StatsMayHaveLanded(_)))
    }

    // An earlier send of this row went unanswered, so the portal may have
    // saved it: its "duplicate" answer now means the stats are already up.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn retry_resolves_on_duplicate_only_after_an_unanswered_send() {
        let (id, events) =
            retry_stats_with(true, Err(PortalCallError::Failed(DUP_BODY.into()))).await;
        assert!(
            resolved(&events, &id),
            "a duplicate answer after an unanswered send must resolve the item, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated)),
            "an already-stored answer must not emit ItemUpdated, got {events:?}"
        );
    }

    // The game-ended-twice case: no earlier send went unanswered, so the
    // duplicate is an OLD event inside a new upload that saved nothing.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn retry_keeps_the_row_on_duplicate_without_an_unanswered_send() {
        let (id, events) =
            retry_stats_with(false, Err(PortalCallError::Failed(DUP_BODY.into()))).await;
        assert!(
            !resolved(&events, &id),
            "a duplicate answer with no unanswered send must NOT resolve the item, got {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated)),
            "the row must stay stats-pending (ItemUpdated), got {events:?}"
        );
    }

    // An unmarked row's first send has no unanswered predecessor, so a
    // duplicate answer to it can never mean "already saved".
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn first_send_answered_duplicate_stays_stats_pending() {
        let (id, events) =
            first_attempt_with_stats(Err(PortalCallError::Failed(DUP_BODY.into()))).await;
        assert!(
            events.iter().any(|ev| matches!(
                ev, PortalEvent::ScoreSentStatsPending(i) if i == &id
            )),
            "a duplicate answer to a first send must leave the item stats-pending, got {events:?}"
        );
        assert!(
            !resolved(&events, &id),
            "a duplicate answer to a first send must NOT resolve the item, got {events:?}"
        );
    }

    // The app closed after an unanswered send was marked but before the row
    // turned stats-pending, so the row goes through a first send again. The
    // mark, not the path, decides: the duplicate answer counts as saved.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn first_send_of_a_marked_row_resolves_on_duplicate() {
        let (id, events) =
            first_attempt_of(true, Err(PortalCallError::Failed(DUP_BODY.into()))).await;
        assert!(
            resolved(&events, &id),
            "a duplicate answer to a marked row's first send must resolve it, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ScoreSentStatsPending(_))),
            "a marked row's duplicate answer must NOT leave it stats-pending, got {events:?}"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn unanswered_stats_send_marks_the_row() {
        // First send: the flag is marked BEFORE the row turns stats-pending,
        // so the persisted row carries it when the operator retries.
        let (id, events) =
            first_attempt_with_stats(Err(PortalCallError::Unreachable("timed out".into()))).await;
        let marked = events
            .iter()
            .position(|ev| matches!(ev, PortalEvent::StatsMayHaveLanded(i) if i == &id));
        let pending = events
            .iter()
            .position(|ev| matches!(ev, PortalEvent::ScoreSentStatsPending(i) if i == &id));
        assert!(
            matches!((marked, pending), (Some(m), Some(p)) if m < p),
            "an unanswered first send must emit StatsMayHaveLanded then ScoreSentStatsPending, got {events:?}"
        );

        // Retry: marked as well, and the row still stays stats-pending.
        let (id, events) =
            retry_stats_with(false, Err(PortalCallError::Unreachable("timed out".into()))).await;
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::StatsMayHaveLanded(i) if i == &id)),
            "an unanswered retry must emit StatsMayHaveLanded, got {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::ItemUpdated)),
            "an unanswered retry must keep the row stats-pending, got {events:?}"
        );
    }

    // The portal answered (`Failed`) or nothing left the machine (`NotSent`):
    // neither can have saved anything unheard, so neither marks the row.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn answered_or_unsent_stats_failure_does_not_mark_the_row() {
        let errors: [fn() -> PortalCallError; 2] = [
            || PortalCallError::Failed("server error".into()),
            || PortalCallError::NotSent("not an https address".into()),
        ];
        for err in errors {
            let (_, events) = first_attempt_with_stats(Err(err())).await;
            assert!(
                !marked_may_have_landed(&events),
                "first send must not mark the row, got {events:?}"
            );
            assert!(
                events
                    .iter()
                    .any(|ev| matches!(ev, PortalEvent::ScoreSentStatsPending(_))),
                "first send must still leave the row stats-pending, got {events:?}"
            );
            let (_, events) = retry_stats_with(false, Err(err())).await;
            assert!(
                !marked_may_have_landed(&events),
                "retry must not mark the row, got {events:?}"
            );
        }
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
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let handle = spawn(io);

        // First: send an empty queue and advance past a tick. No
        // retry-eligible item exists, so no scores/stats calls.
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: QueueFile::empty(),
                epoch: 0,
            })
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
            .send(PortalCommand::QueueUpdated {
                queue: queue_with_one_eligible_item(),
                epoch: 0,
            })
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

        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn failing_item_is_attempted_once_then_throttled_not_every_poll() {
        // Regression for the retry-flood (H4): a failing item used to be
        // re-sent on every POLL_INTERVAL (~2s) tick because `last_attempt_at`
        // was never stamped, so `is_item_retry_eligible` saw `None` forever.
        // The task now stamps it on its own snapshot copy, so within one
        // ITEM_RETRY_INTERVAL the item is attempted exactly once. (Wall-clock
        // barely advances under paused time, so the 15s throttle stays closed
        // across the poll ticks below.)
        let scores_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let io = FakeIo {
            verify_results: Mutex::new(vec![]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![
                Err(PortalCallError::Failed("x".into())),
                Err(PortalCallError::Failed("x".into())),
                Err(PortalCallError::Failed("x".into())),
                Err(PortalCallError::Failed("x".into())),
            ]),
            scores_count: scores_count.clone(),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let handle = spawn(io);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: queue_with_one_eligible_item(),
                epoch: 0,
            })
            .await
            .unwrap();

        // Fire several poll ticks, all within ITEM_RETRY_INTERVAL.
        for _ in 0..4 {
            tokio::task::yield_now().await;
            tokio::time::advance(Duration::from_secs(2)).await;
            tokio::task::yield_now().await;
            tokio::task::yield_now().await;
        }

        assert_eq!(
            scores_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "a failing item must be attempted once then throttled, not re-sent on every poll"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn attempt_emits_item_attempted_event_with_incremented_count() {
        let io = FakeIo {
            verify_results: Mutex::new(vec![]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![Err(PortalCallError::Failed("x".into()))]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: Default::default(),
            stats_epochs: Default::default(),
        };
        let mut handle = spawn(io);
        let queue = queue_with_one_eligible_item();
        let expected_id = queue.items[0].id.clone();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                PortalEvent::ItemAttempted { id, attempts, .. }
                    if id == &expected_id && *attempts == 1
            )),
            "an attempt must emit ItemAttempted with the item id and attempts == 1, got {events:?}"
        );
        drop(handle);
    }

    /// A `FakeIo` whose every call succeeds, plus the epoch logs it records into.
    fn epoch_recording_io() -> (FakeIo, EpochLog, EpochLog) {
        let scores_epochs: EpochLog = Default::default();
        let stats_epochs: EpochLog = Default::default();
        let io = FakeIo {
            verify_results: Mutex::new(vec![]),
            verify_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_results: Mutex::new(vec![]),
            scores_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            stats_results: Mutex::new(vec![]),
            stats_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            scores_epochs: scores_epochs.clone(),
            stats_epochs: stats_epochs.clone(),
        };
        (io, scores_epochs, stats_epochs)
    }

    /// Let queued commands be received, then run one retry tick.
    async fn run_one_tick() {
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn each_post_carries_the_epoch_of_its_snapshot() {
        let (io, scores_epochs, stats_epochs) = epoch_recording_io();
        let handle = spawn(io);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: queue_with_one_eligible_item(),
                epoch: 7,
            })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(*scores_epochs.lock().unwrap(), vec![7]);
        assert_eq!(*stats_epochs.lock().unwrap(), vec![7]);
        drop(handle);
    }

    /// Snapshots are each sent from their own spawned task, so one taken before a discard can
    /// arrive after it. It must not bring the discarded results back.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn an_older_snapshot_never_replaces_a_newer_one() {
        let (io, scores_epochs, stats_epochs) = epoch_recording_io();
        let handle = spawn(io);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: QueueFile::empty(),
                epoch: 2,
            })
            .await
            .unwrap();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: queue_with_one_eligible_item(),
                epoch: 1,
            })
            .await
            .unwrap();

        run_one_tick().await;

        assert!(
            scores_epochs.lock().unwrap().is_empty(),
            "a stale snapshot's item must not be posted"
        );
        assert!(stats_epochs.lock().unwrap().is_empty());
        drop(handle);
    }

    fn mk_swap(event: &str, game: &str) -> QueuedSwap {
        QueuedSwap {
            id: ItemId {
                event_id: event.into(),
                game_number: game.into(),
            },
            dark_team_id: "teams/12-B".into(),
            light_team_id: "teams/12-A".into(),
            queued_at: OffsetDateTime::now_utc(),
            attempts: 0,
            last_attempt_at: None,
        }
    }

    fn mk_score(event: &str, game: &str) -> QueuedItem {
        let mut item = mk_queue_item(0);
        item.id = ItemId {
            event_id: event.into(),
            game_number: game.into(),
        };
        item
    }

    #[test]
    fn score_waits_only_for_its_own_game() {
        let now = OffsetDateTime::now_utc();
        let swaps = vec![mk_swap("e", "12")];
        assert!(score_waits_for_swap(&mk_score("e", "12"), &swaps, now));
        assert!(!score_waits_for_swap(&mk_score("e", "13"), &swaps, now));
        assert!(!score_waits_for_swap(&mk_score("other", "12"), &swaps, now));
        assert!(!score_waits_for_swap(&mk_score("e", "12"), &[], now));
    }

    #[test]
    fn stuck_swap_stops_holding_its_score() {
        let now = OffsetDateTime::now_utc();
        let mut stuck = mk_swap("e", "12");
        stuck.queued_at = now - TimeDuration::minutes(31);
        assert!(
            !score_waits_for_swap(&mk_score("e", "12"), &[stuck.clone()], now),
            "a swap stuck 30 minutes must no longer hold its game's score"
        );
        assert!(
            is_swap_stuck(&stuck, now),
            "the swap itself stays stuck (red, retryable by RETRY ALL)"
        );
        assert!(
            score_waits_for_swap(&mk_score("e", "12"), &[mk_swap("e", "12")], now),
            "a fresh swap still holds its game's score"
        );
    }

    #[test]
    fn swap_step_sorts_outcomes() {
        assert_eq!(
            swap_step(&Ok(ColourSwapOutcome::Applied)),
            SwapStep::Delivered
        );
        assert_eq!(
            swap_step(&Ok(ColourSwapOutcome::Refused {
                status: 409,
                reason: "Game has already started".into(),
            })),
            SwapStep::Dropped
        );
        assert_eq!(
            swap_step(&Ok(ColourSwapOutcome::NotAccepted { status: 404 })),
            SwapStep::Dropped
        );
        assert_eq!(
            swap_step(&Ok(ColourSwapOutcome::Retry { status: 503 })),
            SwapStep::Retry
        );
        assert_eq!(
            swap_step(&Err(PortalCallError::Unreachable("timed out".into()))),
            SwapStep::Retry
        );
        assert_eq!(
            swap_step(&Err(PortalCallError::Failed("no access key".into()))),
            SwapStep::Retry
        );
        assert_eq!(
            swap_step(&Err(PortalCallError::NotSent("discarded".into()))),
            SwapStep::Retry
        );
    }

    /// A test IO for the swap tests: every colours call gets the next queued
    /// answer (`Applied` once they run out), and every score post records its
    /// game number.
    struct SwapIo {
        colours_results: Mutex<Vec<Result<ColourSwapOutcome, PortalCallError>>>,
        colours_count: Arc<std::sync::atomic::AtomicU32>,
        scored_games: Arc<Mutex<Vec<String>>>,
    }

    impl SwapIo {
        fn answering(
            answers: Vec<Result<ColourSwapOutcome, PortalCallError>>,
        ) -> (
            Self,
            Arc<std::sync::atomic::AtomicU32>,
            Arc<Mutex<Vec<String>>>,
        ) {
            let colours_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
            let scored_games: Arc<Mutex<Vec<String>>> = Default::default();
            (
                Self {
                    colours_results: Mutex::new(answers),
                    colours_count: colours_count.clone(),
                    scored_games: scored_games.clone(),
                },
                colours_count,
                scored_games,
            )
        }
    }

    #[async_trait::async_trait]
    impl PortalTaskIo for SwapIo {
        async fn verify_token(&self) -> Result<(), PortalCallError> {
            Ok(())
        }
        async fn post_scores(&self, item: &QueuedItem, _: u64) -> Result<(), PortalCallError> {
            self.scored_games
                .lock()
                .unwrap()
                .push(item.id.game_number.clone());
            Ok(())
        }
        async fn post_stats(&self, _: &QueuedItem, _: u64) -> Result<(), PortalCallError> {
            Ok(())
        }
        async fn post_colours(
            &self,
            _: &QueuedSwap,
            _: u64,
        ) -> Result<ColourSwapOutcome, PortalCallError> {
            self.colours_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = self.colours_results.lock().unwrap();
            if v.is_empty() {
                Ok(ColourSwapOutcome::Applied)
            } else {
                v.remove(0)
            }
        }
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn swap_not_accepted_releases_the_held_score() {
        let (io, colours_count, scored_games) =
            SwapIo::answering(vec![Ok(ColourSwapOutcome::NotAccepted { status: 404 })]);
        let mut handle = spawn(io);
        let swap = mk_swap("e", "12");
        let swap_id = swap.id.clone();
        let (dark, light) = (swap.dark_team_id.clone(), swap.light_team_id.clone());
        let mut queue = QueueFile::empty();
        queue.items.push(mk_score("e", "12"));
        queue.swaps.push(swap);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: queue.clone(),
                epoch: 0,
            })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(colours_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(
            scored_games.lock().unwrap().is_empty(),
            "the score must not go before its swap is resolved"
        );
        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                PortalEvent::SwapResolved {
                    id,
                    dark_team_id,
                    light_team_id,
                    delivered: false,
                } if id == &swap_id && dark_team_id == &dark && light_team_id == &light
            )),
            "a site with no colours call must resolve the swap undelivered, carrying the \
             queued swap's dark and light teams, got {events:?}"
        );

        // The main thread drops the resolved swap and pushes a fresh snapshot.
        queue.swaps.clear();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(*scored_games.lock().unwrap(), vec!["12".to_string()]);
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn score_is_held_while_its_swap_retries() {
        let (io, colours_count, scored_games) =
            SwapIo::answering(vec![Ok(ColourSwapOutcome::Retry { status: 503 })]);
        let mut handle = spawn(io);
        let swap = mk_swap("e", "12");
        let swap_id = swap.id.clone();
        let (dark, light) = (swap.dark_team_id.clone(), swap.light_team_id.clone());
        let mut queue = QueueFile::empty();
        queue.items.push(mk_score("e", "12"));
        queue.items.push(mk_score("e", "13"));
        queue.swaps.push(swap);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(colours_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            *scored_games.lock().unwrap(),
            vec!["13".to_string()],
            "game 12's score waits for its swap; game 13's goes in the same tick"
        );
        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                PortalEvent::SwapAttempted {
                    id,
                    dark_team_id,
                    light_team_id,
                    attempts: 1,
                    ..
                } if id == &swap_id && dark_team_id == &dark && light_team_id == &light
            )),
            "a swap to retry must report its attempt, carrying the queued swap's dark and \
             light teams, got {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, PortalEvent::SwapResolved { .. })),
            "a swap to retry must stay queued, got {events:?}"
        );
        drop(handle);
    }

    #[test]
    fn swap_hold_limit_is_below_the_stuck_threshold() {
        // A held score must be released while it still retries by itself,
        // never after it has turned red.
        assert!(SWAP_HOLD_LIMIT < super::super::STUCK_THRESHOLD);
    }

    /// Timeline (a): the swap was saved at kick-off and the refbox went
    /// offline. 45 minutes later the swap is long stuck; the score was queued
    /// 5 minutes ago, which re-armed the swap (`enqueue_game_end`). When the
    /// connection returns, the swap is sent first and the score waits for it.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn rearmed_swap_goes_before_its_score_after_a_long_outage() {
        let now = OffsetDateTime::now_utc();
        let mut swap = mk_swap("e", "12");
        swap.queued_at = now - TimeDuration::minutes(45);
        swap.attempts = 40;
        swap.last_attempt_at = Some(now - TimeDuration::minutes(16));
        assert!(
            !is_swap_retry_eligible(&swap, now),
            "before the re-arm, the swap saved 45 minutes ago is stuck"
        );
        let mut score = mk_score("e", "12");
        score.queued_at = now - TimeDuration::minutes(5);
        // The re-arm `enqueue_game_end` applied when the score was queued.
        swap.queued_at = score.queued_at;
        swap.attempts = 0;
        swap.last_attempt_at = None;
        assert!(is_swap_retry_eligible(&swap, now));

        let (io, colours_count, scored_games) =
            SwapIo::answering(vec![Ok(ColourSwapOutcome::Retry { status: 503 })]);
        let mut handle = spawn(io);
        let swap_id = swap.id.clone();
        let mut queue = QueueFile::empty();
        queue.items.push(score);
        queue.swaps.push(swap);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated {
                queue: queue.clone(),
                epoch: 0,
            })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(
            colours_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the re-armed swap must be sent when the connection returns"
        );
        assert!(
            scored_games.lock().unwrap().is_empty(),
            "the score must wait while its swap is still pending"
        );
        let events = drain_events(&mut handle.event_rx);
        assert!(
            events.iter().any(|ev| matches!(
                ev,
                PortalEvent::SwapAttempted { id, attempts: 1, .. } if id == &swap_id
            )),
            "got {events:?}"
        );

        // The swap is recorded on the next try; the main thread drops it and
        // pushes a fresh snapshot, and only then does the score go.
        queue.swaps.clear();
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(*scored_games.lock().unwrap(), vec!["12".to_string()]);
        drop(handle);
    }

    /// Timeline (b): the swap and the score were queued within a second of
    /// each other (end-of-game DONE) 21 minutes ago, and the colours call
    /// keeps answering "try again". The score is released by
    /// `SWAP_HOLD_LIMIT` and sent; neither item is stuck yet.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn score_is_released_after_the_swap_hold_limit() {
        let now = OffsetDateTime::now_utc();
        let mut swap = mk_swap("e", "12");
        swap.queued_at = now - TimeDuration::minutes(21);
        swap.attempts = 80;
        swap.last_attempt_at = Some(now - TimeDuration::seconds(20));
        let mut score = mk_score("e", "12");
        score.queued_at = now - TimeDuration::minutes(21);
        assert!(!is_swap_stuck(&swap, now), "the swap is not stuck yet");
        assert!(!is_item_stuck(&score, now), "the score is not stuck yet");
        assert!(
            !score_waits_for_swap(&score, std::slice::from_ref(&swap), now),
            "a score queued 21 minutes ago no longer waits for its swap"
        );

        let (io, colours_count, scored_games) =
            SwapIo::answering(vec![Ok(ColourSwapOutcome::Retry { status: 503 })]);
        let handle = spawn(io);
        let mut queue = QueueFile::empty();
        queue.items.push(score);
        queue.swaps.push(swap);
        handle
            .command_tx
            .send(PortalCommand::QueueUpdated { queue, epoch: 0 })
            .await
            .unwrap();

        run_one_tick().await;

        assert_eq!(
            colours_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the swap still retries by itself"
        );
        assert_eq!(
            *scored_games.lock().unwrap(),
            vec!["12".to_string()],
            "the score goes once it has waited SWAP_HOLD_LIMIT"
        );
        drop(handle);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn retry_stats_carries_its_epoch() {
        let (io, scores_epochs, stats_epochs) = epoch_recording_io();
        let handle = spawn(io);
        let mut item = mk_queue_item(0);
        item.score_sent = true;
        handle
            .command_tx
            .send(PortalCommand::RetryStats { item, epoch: 3 })
            .await
            .unwrap();

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        assert_eq!(*stats_epochs.lock().unwrap(), vec![3]);
        assert!(scores_epochs.lock().unwrap().is_empty());
        drop(handle);
    }
}
