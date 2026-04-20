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
use std::time::Instant;

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

/// Stub for the manager struct — fleshed out in later tasks.
pub struct PortalManager {
    // Fields populated in Task 6.
    indicator_state: PortalIndicatorState,
    _recent_successes_instant: Option<Instant>, // silence unused-field warning
}

impl PortalManager {
    pub fn indicator_state(&self) -> PortalIndicatorState {
        self.indicator_state
    }
}
