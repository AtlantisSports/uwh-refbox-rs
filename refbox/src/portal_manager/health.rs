//! Background health-check task and retry loop.
//!
//! The decision logic (when to fire a health check, when to retry an item,
//! what cadence to use) is encapsulated in `HealthDecisionState` and is
//! unit-testable without any async runtime or I/O.

use std::time::{Duration, Instant};

use super::HealthState;

pub const GREEN_CADENCE: Duration = Duration::from_secs(5 * 60);
pub const DEGRADED_CADENCE: Duration = Duration::from_secs(15);

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
